//! The store: the index in memory, the log on disk, and the rule that keeps
//! them in step (SPEC 7, 9, 10, 11).
//!
//! One `RwLock` guards both. A write appends its record and updates the
//! index while it holds the lock exclusively, so the order of records in
//! the log is always the order in which clients saw their writes happen.
//! Reads share the lock.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::ops::Bound;
use std::path::Path;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::log::{COMPACT_FILE, Log, OpenError as LogOpenError, Recovery, Slot};
use crate::record::{self, Key, MAGIC, Value};

/// The lock file inside the data directory (SPEC 4).
pub const LOCK_FILE: &str = "kv.lock";

/// How a store is opened.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Sync the log after every write (SPEC 9).
    pub fsync: bool,
    /// Compact when this many bytes of the log are garbage; 0 means never
    /// (SPEC 10.3).
    pub auto_compact: u64,
}

/// Why a store could not be opened. Every case is a start-up failure with
/// exit code 3.
#[derive(Debug)]
pub enum OpenError {
    /// Another process holds the lock on this directory.
    Locked,
    Log(LogOpenError),
    Io {
        doing: &'static str,
        source: io::Error,
    },
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Locked => write!(f, "another kvstore process is using this data directory"),
            OpenError::Log(e) => e.fmt(f),
            OpenError::Io { doing, source } => write!(f, "{doing} failed: {source}"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Locked => None,
            OpenError::Log(e) => Some(e),
            OpenError::Io { source, .. } => Some(source),
        }
    }
}

impl From<LogOpenError> for OpenError {
    fn from(e: LogOpenError) -> OpenError {
        OpenError::Log(e)
    }
}

/// A failure of the store while it runs. None of these can be repaired by
/// the process: the server stops (SPEC 9.3).
#[derive(Debug)]
pub enum StoreError {
    /// Reading or writing the log failed.
    Io(io::Error),
    /// An earlier write or compaction failed. The log may end in half a
    /// record, so nothing more is written or acknowledged.
    Failed,
    /// A thread panicked while it held the lock.
    Poisoned,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "log I/O failed: {e}"),
            StoreError::Failed => write!(f, "the store has stopped after an earlier I/O failure"),
            StoreError::Poisoned => write!(f, "a thread panicked while it was changing the store"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Io(e) => Some(e),
            StoreError::Failed | StoreError::Poisoned => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> StoreError {
        StoreError::Io(e)
    }
}

/// The result of `INCR` (SPEC 8.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Incremented {
    To(i64),
    NotAnInteger,
    Overflow,
}

/// The numbers of a `STATS` response (SPEC 8.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub keys: u64,
    pub log_bytes: u64,
    pub live_bytes: u64,
    pub compactions: u64,
}

/// The live keys and where their values are, plus the running total that
/// `STATS` and automatic compaction need.
#[derive(Debug)]
struct Index {
    /// A `BTreeMap` because `KEYS` and compaction want byte order, and a
    /// prefix is a range in that order.
    slots: BTreeMap<Box<[u8]>, Slot>,
    /// The size the log would have right after a compaction: the header
    /// plus one SET record per live key.
    live_bytes: u64,
}

impl Index {
    fn new() -> Index {
        Index {
            slots: BTreeMap::new(),
            live_bytes: MAGIC.len() as u64,
        }
    }

    fn get(&self, key: Key<'_>) -> Option<Slot> {
        self.slots.get(key.as_bytes()).copied()
    }

    fn set(&mut self, key: Key<'_>, slot: Slot) {
        let key = key.as_bytes();
        self.live_bytes += record::encoded_len(key.len(), slot.len as usize);
        match self.slots.get_mut(key) {
            // The key is already owned by the map: reuse it.
            Some(old) => {
                self.live_bytes -= record::encoded_len(key.len(), old.len as usize);
                *old = slot;
            }
            None => {
                self.slots.insert(key.into(), slot);
            }
        }
    }

    fn remove(&mut self, key: Key<'_>) -> bool {
        let key = key.as_bytes();
        match self.slots.remove(key) {
            Some(old) => {
                self.live_bytes -= record::encoded_len(key.len(), old.len as usize);
                true
            }
            None => false,
        }
    }

    /// Replays one record of the log.
    fn apply(&mut self, key: Key<'_>, slot: Option<Slot>) {
        match slot {
            Some(slot) => self.set(key, slot),
            None => {
                self.remove(key);
            }
        }
    }
}

#[derive(Debug)]
struct Inner {
    index: Index,
    /// `None` after a failed write or compaction: see [`StoreError::Failed`].
    log: Option<Log>,
    compactions: u64,
}

impl Inner {
    fn log(&self) -> Result<&Log, StoreError> {
        self.log.as_ref().ok_or(StoreError::Failed)
    }

    /// Runs one append. If it fails, the log is dropped for good, so that
    /// no later write can land behind a torn record and be acknowledged.
    fn append<T>(
        &mut self,
        write: impl FnOnce(&mut Log) -> io::Result<T>,
    ) -> Result<T, StoreError> {
        let log = self.log.as_mut().ok_or(StoreError::Failed)?;
        write(log).map_err(|e| {
            self.log = None;
            StoreError::Io(e)
        })
    }

    fn set(&mut self, key: Key<'_>, value: Value<'_>) -> Result<(), StoreError> {
        // Log first, index second: the index must never point at bytes
        // that are not in the file.
        let slot = self.append(|log| log.append_set(key, value))?;
        self.index.set(key, slot);
        Ok(())
    }

    fn garbage(&self) -> u64 {
        self.log
            .as_ref()
            .map_or(0, |log| log.size() - self.index.live_bytes)
    }

    fn compact(&mut self) -> Result<(), StoreError> {
        // Taking the log out means an error leaves `None` behind, which is
        // the failed state.
        let log = self.log.take().ok_or(StoreError::Failed)?;
        let live = self
            .index
            .slots
            .iter()
            .map(|(key, slot)| (key.len(), *slot));
        let (log, slots) = log.compact(live)?;
        // `compact` answers in the order it was asked, which is map order.
        for (slot, moved) in self.index.slots.values_mut().zip(slots) {
            *slot = moved;
        }
        debug_assert_eq!(log.size(), self.index.live_bytes);
        self.log = Some(log);
        self.compactions += 1;
        Ok(())
    }
}

/// A key-value store in one directory. All methods take `&self`; share it
/// between threads by reference.
#[derive(Debug)]
pub struct Store {
    inner: RwLock<Inner>,
    auto_compact: u64,
    /// Holds the lock on the directory. The operating system releases it
    /// when the handle is closed, also when the process is killed.
    _lock: File,
}

impl Store {
    /// Opens the store in `dir`, creating the directory and the log if they
    /// do not exist, and recovers the log (SPEC 6).
    pub fn open(dir: &Path, options: Options) -> Result<(Store, Recovery), OpenError> {
        let io = |doing| move |source| OpenError::Io { doing, source };
        fs::create_dir_all(dir).map_err(io("creating the data directory"))?;

        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join(LOCK_FILE))
            .map_err(io("opening the lock file"))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(OpenError::Locked),
            Err(TryLockError::Error(e)) => return Err(io("locking the data directory")(e)),
        }

        // Only now is it certain that no other server is in the middle of a
        // compaction. A file left here belongs to a compaction that never
        // reached its rename, so the log is still the truth.
        match fs::remove_file(dir.join(COMPACT_FILE)) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(io("removing an unfinished compaction file")(e)),
        }

        let mut index = Index::new();
        let (log, recovery) = Log::open(dir, options.fsync, |key, slot| index.apply(key, slot))?;
        let inner = Inner {
            index,
            log: Some(log),
            compactions: 0,
        };
        let store = Store {
            inner: RwLock::new(inner),
            auto_compact: options.auto_compact,
            _lock: lock,
        };
        Ok((store, recovery))
    }

    fn read(&self) -> Result<RwLockReadGuard<'_, Inner>, StoreError> {
        self.inner.read().map_err(|_| StoreError::Poisoned)
    }

    fn write(&self) -> Result<RwLockWriteGuard<'_, Inner>, StoreError> {
        self.inner.write().map_err(|_| StoreError::Poisoned)
    }

    /// Compacts if the last write pushed the garbage over the threshold.
    /// This runs before the write is acknowledged, which keeps the rule of
    /// SPEC 10.3 trivially; DESIGN.md says what that costs.
    fn after_write(&self, inner: &mut Inner) -> Result<(), StoreError> {
        if self.auto_compact > 0 && inner.garbage() >= self.auto_compact {
            inner.compact()?;
        }
        Ok(())
    }

    pub fn get(&self, key: Key<'_>) -> Result<Option<Vec<u8>>, StoreError> {
        let inner = self.read()?;
        match inner.index.get(key) {
            Some(slot) => Ok(Some(inner.log()?.read_value(slot)?)),
            None => Ok(None),
        }
    }

    pub fn set(&self, key: Key<'_>, value: Value<'_>) -> Result<(), StoreError> {
        let mut inner = self.write()?;
        inner.set(key, value)?;
        self.after_write(&mut inner)
    }

    /// Returns whether the key existed. A missing key writes nothing.
    pub fn delete(&self, key: Key<'_>) -> Result<bool, StoreError> {
        let mut inner = self.write()?;
        if inner.index.get(key).is_none() {
            return Ok(false);
        }
        inner.append(|log| log.append_del(key))?;
        inner.index.remove(key);
        self.after_write(&mut inner)?;
        Ok(true)
    }

    /// Adds 1 to the number stored under `key`. The read and the write
    /// happen under one exclusive lock, so no increment can be lost.
    pub fn incr(&self, key: Key<'_>) -> Result<Incremented, StoreError> {
        let mut inner = self.write()?;
        let current = match inner.index.get(key) {
            Some(slot) => {
                let bytes = inner.log()?.read_value(slot)?;
                // `i64::from_str` accepts exactly the numbers of SPEC 8.7:
                // an optional sign, then digits, within range.
                match std::str::from_utf8(&bytes)
                    .ok()
                    .and_then(|text| text.parse::<i64>().ok())
                {
                    Some(number) => number,
                    None => return Ok(Incremented::NotAnInteger),
                }
            }
            None => 0,
        };
        let Some(next) = current.checked_add(1) else {
            return Ok(Incremented::Overflow);
        };
        let text = next.to_string();
        let value =
            Value::new(text.as_bytes()).expect("a decimal i64 is a short value without line feeds");
        inner.set(key, value)?;
        self.after_write(&mut inner)?;
        Ok(Incremented::To(next))
    }

    /// The live keys that start with `prefix` (all keys if there is none),
    /// in ascending byte order.
    pub fn keys(&self, prefix: Option<Key<'_>>) -> Result<Vec<Box<[u8]>>, StoreError> {
        let inner = self.read()?;
        let prefix = prefix.map_or(&[][..], Key::as_bytes);
        // In byte order, the keys with a prefix are one unbroken run that
        // starts at the first key not smaller than the prefix.
        let keys = inner
            .index
            .slots
            .range::<[u8], _>((Bound::Included(prefix), Bound::Unbounded))
            .take_while(|(key, _)| key.starts_with(prefix))
            .map(|(key, _)| key.clone())
            .collect();
        Ok(keys)
    }

    pub fn stats(&self) -> Result<Stats, StoreError> {
        let inner = self.read()?;
        Ok(Stats {
            keys: inner.index.slots.len() as u64,
            log_bytes: inner.log()?.size(),
            live_bytes: inner.index.live_bytes,
            compactions: inner.compactions,
        })
    }

    /// Rewrites the log to one record per live key (SPEC 10). Other
    /// commands wait while this runs.
    pub fn compact(&self) -> Result<(), StoreError> {
        self.write()?.compact()
    }

    /// Forces the log onto the disk. Called once, at shutdown, whatever the
    /// fsync mode is.
    pub fn sync(&self) -> Result<(), StoreError> {
        Ok(self.read()?.log()?.sync()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let path = std::env::temp_dir()
                .join(format!("kvstore-unit-store-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const QUIET: Options = Options {
        fsync: false,
        auto_compact: 0,
    };

    fn key(text: &str) -> Key<'_> {
        Key::new(text.as_bytes()).unwrap()
    }

    fn value(text: &str) -> Value<'_> {
        Value::new(text.as_bytes()).unwrap()
    }

    fn keys(store: &Store, prefix: Option<&str>) -> Vec<String> {
        store
            .keys(prefix.map(key))
            .unwrap()
            .iter()
            .map(|k| String::from_utf8_lossy(k).into_owned())
            .collect()
    }

    #[test]
    fn set_get_delete_and_reopen() {
        let dir = TempDir::new("basic");
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        assert_eq!(store.get(key("a")).unwrap(), None);
        store.set(key("a"), value("1")).unwrap();
        store.set(key("b"), value("2")).unwrap();
        store.set(key("a"), value("3")).unwrap();
        assert_eq!(store.get(key("a")).unwrap().as_deref(), Some(&b"3"[..]));
        assert!(store.delete(key("b")).unwrap());
        assert!(!store.delete(key("b")).unwrap());
        let before = store.stats().unwrap();
        // 8 + (11+1+1) * 3 + (11+1) = 59; live: 8 + 13 = 21.
        assert_eq!(
            before,
            Stats {
                keys: 1,
                log_bytes: 59,
                live_bytes: 21,
                compactions: 0
            }
        );
        drop(store);

        let (store, recovery) = Store::open(&dir.0, QUIET).unwrap();
        assert_eq!(
            recovery,
            Recovery {
                records: 4,
                discarded: 0
            }
        );
        assert_eq!(store.stats().unwrap(), before);
        assert_eq!(store.get(key("a")).unwrap().as_deref(), Some(&b"3"[..]));
    }

    #[test]
    fn a_second_store_on_the_same_directory_is_refused() {
        let dir = TempDir::new("lock");
        let (first, _) = Store::open(&dir.0, QUIET).unwrap();
        assert!(matches!(Store::open(&dir.0, QUIET), Err(OpenError::Locked)));
        drop(first);
        assert!(
            Store::open(&dir.0, QUIET).is_ok(),
            "closing the first store releases the lock"
        );
    }

    #[test]
    fn keys_by_prefix() {
        let dir = TempDir::new("keys");
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        for k in ["b", "a", "B", "ab", "a1", "~", "abc"] {
            store.set(key(k), value("")).unwrap();
        }
        assert_eq!(keys(&store, None), ["B", "a", "a1", "ab", "abc", "b", "~"]);
        assert_eq!(keys(&store, Some("a")), ["a", "a1", "ab", "abc"]);
        assert_eq!(keys(&store, Some("ab")), ["ab", "abc"]);
        assert_eq!(keys(&store, Some("abcd")), Vec::<String>::new());
        assert_eq!(keys(&store, Some("~")), ["~"]);
    }

    #[test]
    fn incr() {
        let dir = TempDir::new("incr");
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        assert_eq!(store.incr(key("n")).unwrap(), Incremented::To(1));
        assert_eq!(store.incr(key("n")).unwrap(), Incremented::To(2));
        store.set(key("n"), value("x")).unwrap();
        assert_eq!(store.incr(key("n")).unwrap(), Incremented::NotAnInteger);
        store.set(key("n"), value("9223372036854775807")).unwrap();
        let before = store.stats().unwrap();
        assert_eq!(store.incr(key("n")).unwrap(), Incremented::Overflow);
        assert_eq!(
            store.stats().unwrap(),
            before,
            "a failed INCR writes nothing"
        );
    }

    #[test]
    fn compaction_keeps_the_content_and_drops_the_garbage() {
        let dir = TempDir::new("compact");
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        for round in 0..10 {
            for k in ["x", "y", "z"] {
                store.set(key(k), value(&format!("{k}{round}"))).unwrap();
            }
        }
        store.delete(key("y")).unwrap();
        store.compact().unwrap();
        let stats = store.stats().unwrap();
        assert_eq!(stats.log_bytes, stats.live_bytes);
        assert_eq!(stats.live_bytes, 8 + 2 * (11 + 1 + 2));
        assert_eq!(stats.compactions, 1);
        assert_eq!(store.get(key("x")).unwrap().as_deref(), Some(&b"x9"[..]));
        assert_eq!(store.get(key("z")).unwrap().as_deref(), Some(&b"z9"[..]));
        assert_eq!(store.get(key("y")).unwrap(), None);
    }

    #[test]
    fn automatic_compaction_bounds_the_garbage() {
        let dir = TempDir::new("auto");
        let options = Options {
            fsync: false,
            auto_compact: 100,
        };
        let (store, _) = Store::open(&dir.0, options).unwrap();
        for i in 0..200 {
            store.set(key("k"), value(&format!("{i:04}"))).unwrap();
            let stats = store.stats().unwrap();
            assert!(
                stats.log_bytes - stats.live_bytes < 100,
                "after write {i}: {stats:?}"
            );
        }
        assert!(store.stats().unwrap().compactions > 10);
        assert_eq!(store.get(key("k")).unwrap().as_deref(), Some(&b"0199"[..]));
    }

    #[test]
    fn an_unfinished_compaction_file_is_removed() {
        let dir = TempDir::new("leftover");
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        store.set(key("a"), value("1")).unwrap();
        drop(store);
        fs::write(dir.0.join(COMPACT_FILE), b"KVSTORE1 half a compaction").unwrap();
        let (store, _) = Store::open(&dir.0, QUIET).unwrap();
        assert!(!dir.0.join(COMPACT_FILE).exists());
        assert_eq!(store.get(key("a")).unwrap().as_deref(), Some(&b"1"[..]));
    }
}
