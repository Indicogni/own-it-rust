//! The log file: opening and recovery (SPEC 6), appending (SPEC 9), reading
//! values back, and compaction (SPEC 10).
//!
//! This module knows files and offsets. It does not know which keys are
//! live; the caller keeps the index and tells the log what to copy when it
//! compacts.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::record::{self, HEADER_LEN, Key, MAGIC, Value};

/// The log, inside the data directory (SPEC 4).
pub const LOG_FILE: &str = "kv.log";
/// The file a compaction builds before it replaces the log (SPEC 4).
pub const COMPACT_FILE: &str = "kv.compact";

/// Where the value of a live key is in the log file.
///
/// The record that holds the value starts `HEADER_LEN + key length` bytes
/// before `offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub offset: u64,
    pub len: u32,
}

/// What recovery found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recovery {
    /// Valid records that were replayed.
    pub records: u64,
    /// Bytes after the valid prefix that were cut off.
    pub discarded: u64,
}

/// Why a log could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// The file exists but does not start with [`MAGIC`]. It is left alone.
    NotALog(PathBuf),
    Io {
        doing: &'static str,
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::NotALog(path) => write!(
                f,
                "{} does not start with `KVSTORE1`; it is not a log of this program and was not changed",
                path.display()
            ),
            OpenError::Io {
                doing,
                path,
                source,
            } => {
                write!(f, "{doing} {} failed: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::NotALog(_) => None,
            OpenError::Io { source, .. } => Some(source),
        }
    }
}

/// An open log file.
///
/// There are two handles on the file because they have different jobs and
/// different locking needs. `writer` only ever appends and is used through
/// `&mut self`, so the caller's write lock protects it. `reader` serves
/// `GET`s through `&self`, possibly from many threads at once; a read is a
/// seek followed by a read, and the small mutex keeps those two steps of
/// different threads apart. (DESIGN.md discusses positioned reads, which
/// would need no mutex but are not portable in `std`.)
#[derive(Debug)]
pub struct Log {
    dir: PathBuf,
    writer: File,
    reader: Mutex<File>,
    /// The size of the file, which is also where the next record goes.
    len: u64,
    /// Whether every append is followed by `sync_data` (SPEC 9).
    fsync: bool,
    /// The record being encoded; kept to reuse its allocation.
    scratch: Vec<u8>,
}

impl Log {
    /// Opens `dir/kv.log`, creating it if needed, and replays it.
    ///
    /// `replay` is called once per valid record, in file order: with
    /// `Some(slot)` for a SET and `None` for a DEL. Everything after the
    /// valid prefix is cut off before this function returns.
    pub fn open(
        dir: &Path,
        fsync: bool,
        mut replay: impl FnMut(Key<'_>, Option<Slot>),
    ) -> Result<(Log, Recovery), OpenError> {
        let path = dir.join(LOG_FILE);
        let io = |doing: &'static str| {
            let path = path.clone();
            move |source| OpenError::Io {
                doing,
                path,
                source,
            }
        };

        // Not `append(true)`: recovery has to shorten the file, and Windows
        // does not allow that through an append-only handle.
        let mut writer = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(io("opening"))?;
        let file_len = writer.metadata().map_err(io("reading the size of"))?.len();

        let mut input = BufReader::with_capacity(1 << 16, &writer);
        let mut magic = [0u8; MAGIC.len()];
        let got = read_up_to(&mut input, &mut magic).map_err(io("reading"))?;
        let (len, records) = if got == MAGIC.len() && magic == MAGIC {
            scan(&mut input, &mut replay).map_err(io("reading"))?
        } else if got < MAGIC.len() && MAGIC.starts_with(&magic[..got]) {
            // An empty file, or one that died while its header was written.
            (0, 0)
        } else {
            return Err(OpenError::NotALog(path));
        };
        drop(input);

        if len < file_len {
            writer.set_len(len).map_err(io("truncating"))?;
        }
        writer
            .seek(SeekFrom::Start(len))
            .map_err(io("seeking in"))?;
        let mut len = len;
        if len == 0 {
            writer
                .write_all(&MAGIC)
                .map_err(io("writing the header of"))?;
            len = MAGIC.len() as u64;
        }
        if len != file_len {
            // The repair itself must survive a power failure.
            writer.sync_all().map_err(io("syncing"))?;
            sync_dir(dir).map_err(io("syncing the directory of"))?;
        }

        let reader = File::open(&path).map_err(io("opening"))?;
        let log = Log {
            dir: dir.to_owned(),
            writer,
            reader: Mutex::new(reader),
            len,
            fsync,
            scratch: Vec::with_capacity(HEADER_LEN + record::MAX_KEY + record::MAX_VALUE),
        };
        let recovery = Recovery {
            records,
            // A torn header is at most 7 bytes, replaced by 8: nothing lost.
            discarded: file_len.saturating_sub(len),
        };
        Ok((log, recovery))
    }

    /// The size of the log file in bytes. Never less than the 8 bytes of
    /// the header.
    pub fn size(&self) -> u64 {
        self.len
    }

    /// Appends a SET record and says where its value is.
    ///
    /// When this returns `Ok`, the record has been handed to the operating
    /// system in one `write`, and has been synced if the log was opened
    /// with `fsync`. After an `Err` the file may end in half a record; the
    /// caller must not append again.
    pub fn append_set(&mut self, key: Key<'_>, value: Value<'_>) -> io::Result<Slot> {
        let start = self.append(key, Some(value))?;
        Ok(Slot {
            offset: start + (HEADER_LEN + key.as_bytes().len()) as u64,
            len: value.as_bytes().len() as u32,
        })
    }

    /// Appends a DEL record. See [`Log::append_set`] for the guarantees.
    pub fn append_del(&mut self, key: Key<'_>) -> io::Result<()> {
        self.append(key, None).map(drop)
    }

    fn append(&mut self, key: Key<'_>, value: Option<Value<'_>>) -> io::Result<u64> {
        self.scratch.clear();
        record::encode(&mut self.scratch, key, value);
        // No `BufWriter`: the bytes must be in the kernel, not in this
        // process, before anyone is told `OK`.
        self.writer.write_all(&self.scratch)?;
        if self.fsync {
            self.writer.sync_data()?;
        }
        let start = self.len;
        self.len += self.scratch.len() as u64;
        Ok(start)
    }

    /// Reads a value back from the file.
    pub fn read_value(&self, slot: Slot) -> io::Result<Vec<u8>> {
        let mut value = vec![0; slot.len as usize];
        self.read_at(slot.offset, &mut value)?;
        Ok(value)
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        // A panic while the mutex was held cannot leave the file handle in
        // a bad state: the next read seeks again.
        let mut reader = self.reader.lock().unwrap_or_else(|e| e.into_inner());
        reader.seek(SeekFrom::Start(offset))?;
        reader.read_exact(buf)
    }

    /// Forces everything appended so far onto the disk.
    pub fn sync(&self) -> io::Result<()> {
        self.writer.sync_all()
    }

    /// Rewrites the log so that it holds exactly the records of `live`, in
    /// that order, and returns the new log with the new place of each value.
    ///
    /// `live` gives, for each live key, the length of the key and the slot
    /// of its value. The records are copied byte for byte, checksum
    /// included, and each one is verified on the way: a record that has
    /// gone bad on disk since it was written stops the compaction instead
    /// of being carried over.
    ///
    /// The old log is replaced with one `rename`, which is the moment the
    /// compaction takes effect (SPEC 10.2). A crash before it leaves the
    /// old log, a crash after it the new one. After an `Err` the log is
    /// gone from this process; the files on disk are still one of the two.
    pub fn compact(
        self,
        live: impl Iterator<Item = (usize, Slot)>,
    ) -> io::Result<(Log, Vec<Slot>)> {
        let log_path = self.dir.join(LOG_FILE);
        let tmp_path = self.dir.join(COMPACT_FILE);

        let mut out = BufWriter::with_capacity(1 << 16, File::create(&tmp_path)?);
        out.write_all(&MAGIC)?;
        let mut len = MAGIC.len() as u64;
        let mut slots = Vec::with_capacity(live.size_hint().0);
        let mut record = Vec::new();
        for (key_len, slot) in live {
            let head_len = (HEADER_LEN + key_len) as u64;
            record.resize(HEADER_LEN + key_len + slot.len as usize, 0);
            self.read_at(slot.offset - head_len, &mut record)?;
            let intact = record
                .split_first_chunk::<HEADER_LEN>()
                .is_some_and(|(header, body)| record::decode(header, body).is_some());
            if !intact {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "the record of a live key at offset {} is damaged",
                        slot.offset - head_len
                    ),
                ));
            }
            out.write_all(&record)?;
            slots.push(Slot {
                offset: len + head_len,
                len: slot.len,
            });
            len += record.len() as u64;
        }
        let tmp = out.into_inner().map_err(io::IntoInnerError::into_error)?;
        tmp.sync_all()?;

        // Close every handle before the rename: Windows may refuse to
        // replace a file that is open.
        drop(tmp);
        let Log {
            dir,
            writer,
            reader,
            fsync,
            scratch,
            ..
        } = self;
        drop(writer);
        drop(reader);
        fs::rename(&tmp_path, &log_path)?;
        sync_dir(&dir)?;

        let mut writer = OpenOptions::new().read(true).write(true).open(&log_path)?;
        writer.seek(SeekFrom::Start(len))?;
        let reader = Mutex::new(File::open(&log_path)?);
        Ok((
            Log {
                dir,
                writer,
                reader,
                len,
                fsync,
                scratch,
            },
            slots,
        ))
    }
}

/// Reads records until the first one that is not valid (SPEC 6). Returns
/// where the valid prefix ends and how many records it holds.
fn scan(
    input: &mut impl Read,
    replay: &mut impl FnMut(Key<'_>, Option<Slot>),
) -> io::Result<(u64, u64)> {
    let mut at = MAGIC.len() as u64;
    let mut records = 0;
    let mut header = [0u8; HEADER_LEN];
    let mut body = Vec::new();
    loop {
        if !read_full(input, &mut header)? {
            break;
        }
        // Check the lengths before believing them: a damaged header must
        // not make us allocate or read gigabytes.
        let Some(body_len) = record::body_len(&header) else {
            break;
        };
        body.resize(body_len, 0);
        if !read_full(input, &mut body)? {
            break;
        }
        let Some((key, value)) = record::decode(&header, &body) else {
            break;
        };
        let slot = value.map(|value| Slot {
            offset: at + (HEADER_LEN + key.as_bytes().len()) as u64,
            len: value.as_bytes().len() as u32,
        });
        replay(key, slot);
        at += (HEADER_LEN + body_len) as u64;
        records += 1;
    }
    Ok((at, records))
}

/// Fills `buf` completely, or reports `false` if the input ends first.
fn read_full(input: &mut impl Read, buf: &mut [u8]) -> io::Result<bool> {
    match input.read_exact(buf) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e),
    }
}

/// Reads until `buf` is full or the input ends; returns the bytes read.
fn read_up_to(input: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match input.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(got)
}

/// Makes a creation or a rename inside `dir` durable. On Unix the name of a
/// file lives in its directory, and the directory has to be synced like a
/// file.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// Other systems give `std` no way to sync a directory.
#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A directory under the system's temporary directory, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let path =
                std::env::temp_dir().join(format!("kvstore-unit-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    type Index = BTreeMap<Vec<u8>, Slot>;

    fn open(dir: &TempDir) -> (Log, Recovery, Index) {
        let mut index = Index::new();
        let (log, recovery) = Log::open(&dir.0, false, |key, slot| match slot {
            Some(slot) => {
                index.insert(key.as_bytes().to_vec(), slot);
            }
            None => {
                index.remove(key.as_bytes());
            }
        })
        .unwrap();
        (log, recovery, index)
    }

    fn key(text: &str) -> Key<'_> {
        Key::new(text.as_bytes()).unwrap()
    }

    fn value(text: &str) -> Value<'_> {
        Value::new(text.as_bytes()).unwrap()
    }

    #[test]
    fn a_new_log_is_just_the_header() {
        let dir = TempDir::new("new");
        let (log, recovery, index) = open(&dir);
        assert_eq!(log.size(), 8);
        assert_eq!(
            recovery,
            Recovery {
                records: 0,
                discarded: 0
            }
        );
        assert!(index.is_empty());
        assert_eq!(fs::read(dir.0.join(LOG_FILE)).unwrap(), MAGIC);
    }

    #[test]
    fn appended_values_can_be_read_back_and_replayed() {
        let dir = TempDir::new("append");
        let (mut log, ..) = open(&dir);
        let a = log.append_set(key("a"), value("first")).unwrap();
        let b = log.append_set(key("bb"), value("")).unwrap();
        log.append_del(key("a")).unwrap();
        let c = log.append_set(key("c"), value("third")).unwrap();
        assert_eq!(log.read_value(a).unwrap(), b"first");
        assert_eq!(log.read_value(b).unwrap(), b"");
        assert_eq!(log.read_value(c).unwrap(), b"third");
        let len = log.size();
        assert_eq!(len, fs::metadata(dir.0.join(LOG_FILE)).unwrap().len());
        drop(log);

        let (log, recovery, index) = open(&dir);
        assert_eq!(
            recovery,
            Recovery {
                records: 4,
                discarded: 0
            }
        );
        assert_eq!(log.size(), len);
        assert_eq!(
            index,
            Index::from([(b"bb".to_vec(), b), (b"c".to_vec(), c)])
        );
    }

    #[test]
    fn a_torn_tail_is_cut_off_at_every_length() {
        let dir = TempDir::new("torn");
        let (mut log, ..) = open(&dir);
        log.append_set(key("keep"), value("me")).unwrap();
        let good = log.size();
        log.append_set(key("lose"), value("this")).unwrap();
        let full = log.size();
        drop(log);
        let path = dir.0.join(LOG_FILE);
        let bytes = fs::read(&path).unwrap();
        for cut in good..full {
            fs::write(&path, &bytes[..cut as usize]).unwrap();
            let (log, recovery, index) = open(&dir);
            assert_eq!(log.size(), good, "cut at {cut}");
            assert_eq!(
                recovery,
                Recovery {
                    records: 1,
                    discarded: cut - good
                },
                "cut at {cut}"
            );
            assert_eq!(index.len(), 1, "cut at {cut}");
            drop(log);
            assert_eq!(
                fs::metadata(&path).unwrap().len(),
                good,
                "cut at {cut}: file not truncated"
            );
        }
    }

    #[test]
    fn a_torn_header_starts_a_new_log_and_a_foreign_file_is_refused() {
        let dir = TempDir::new("header");
        let path = dir.0.join(LOG_FILE);
        for cut in 0..MAGIC.len() {
            fs::write(&path, &MAGIC[..cut]).unwrap();
            let (log, ..) = open(&dir);
            assert_eq!(log.size(), 8, "cut at {cut}");
            drop(log);
            assert_eq!(fs::read(&path).unwrap(), MAGIC, "cut at {cut}");
        }
        for foreign in [&b"hello"[..], b"KVSTORE2", b"KVSTORE2 and more", b"KVS\n"] {
            fs::write(&path, foreign).unwrap();
            let result = Log::open(&dir.0, false, |_, _| {});
            assert!(matches!(result, Err(OpenError::NotALog(_))), "{foreign:?}");
            assert_eq!(
                fs::read(&path).unwrap(),
                foreign,
                "a foreign file must not be changed"
            );
        }
    }

    #[test]
    fn appends_after_recovery_start_at_the_cut() {
        let dir = TempDir::new("after-cut");
        let (mut log, ..) = open(&dir);
        log.append_set(key("a"), value("1")).unwrap();
        let good = log.size();
        log.append_set(key("b"), value("2")).unwrap();
        log.append_set(key("c"), value("3")).unwrap();
        drop(log);
        let path = dir.0.join(LOG_FILE);
        let mut bytes = fs::read(&path).unwrap();
        bytes[good as usize + 5] ^= 1; // damage record b; c is behind it
        fs::write(&path, &bytes).unwrap();

        let (mut log, recovery, _) = open(&dir);
        assert_eq!(recovery.records, 1);
        assert_eq!(log.size(), good);
        let d = log.append_set(key("d"), value("4")).unwrap();
        assert_eq!(d.offset, good + 12);
        drop(log);
        let (_, recovery, index) = open(&dir);
        assert_eq!(
            recovery,
            Recovery {
                records: 2,
                discarded: 0
            }
        );
        assert_eq!(
            index.keys().collect::<Vec<_>>(),
            [b"a", b"d"],
            "record c must not come back"
        );
    }

    #[test]
    fn compaction_copies_live_records_in_the_given_order() {
        let dir = TempDir::new("compact");
        let (mut log, ..) = open(&dir);
        log.append_set(key("b"), value("old")).unwrap();
        let a = log.append_set(key("a"), value("alpha")).unwrap();
        let b = log.append_set(key("b"), value("beta")).unwrap();
        log.append_set(key("gone"), value("x")).unwrap();
        log.append_del(key("gone")).unwrap();

        let (mut log, slots) = log.compact([(1, a), (1, b)].into_iter()).unwrap();
        let mut expected = MAGIC.to_vec();
        record::encode(&mut expected, key("a"), Some(value("alpha")));
        record::encode(&mut expected, key("b"), Some(value("beta")));
        let path = dir.0.join(LOG_FILE);
        assert_eq!(fs::read(&path).unwrap(), expected);
        assert_eq!(log.size(), expected.len() as u64);
        assert!(!dir.0.join(COMPACT_FILE).exists());
        assert_eq!(log.read_value(slots[0]).unwrap(), b"alpha");
        assert_eq!(log.read_value(slots[1]).unwrap(), b"beta");

        // The new log takes appends where it ends.
        let c = log.append_set(key("c"), value("gamma")).unwrap();
        assert_eq!(log.read_value(c).unwrap(), b"gamma");
        drop(log);
        let (_, recovery, index) = open(&dir);
        assert_eq!(
            recovery,
            Recovery {
                records: 3,
                discarded: 0
            }
        );
        assert_eq!(index.len(), 3);
    }
}
