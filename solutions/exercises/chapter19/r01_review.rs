//! REVIEW 01 — Spaced retrieval: Chapters 14–18 inside Chapter 19
//! Run: cargo test --test chapter19 r01_
//!
//! Do this WITHOUT reopening Chapters 14–18 or their hints. Struggling to recall
//! is what makes the knowledge stick; look things up only after a real attempt,
//! and note in NOTES.md anything you had to look up.
//!
//! Each task mixes an earlier idea (readers, maps, boxed closures) with this
//! chapter's threads.

use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, Cursor, Read},
    sync::Mutex,
    thread,
};

/// A. (Ch18 buffered readers + Ch19 spawn/join) Move each source into its own
/// worker, which wraps it in a `BufReader` and counts its lines with
/// `BufRead::lines`. Invalid UTF-8 is that source's error. All reading happens
/// on the workers. Results come back in input order.
fn line_counts<R: Read + Send + 'static>(sources: Vec<R>) -> Vec<io::Result<usize>> {
    let workers: Vec<_> = sources
        .into_iter()
        .map(|source| {
            thread::spawn(move || {
                let mut lines = 0;
                for line in BufReader::new(source).lines() {
                    // `?` works here because the closure returns io::Result.
                    line?;
                    lines += 1;
                }
                Ok(lines)
            })
        })
        .collect();
    workers
        .into_iter()
        .map(|worker| worker.join().expect("line counter panicked"))
        .collect()
}

/// B. (Ch16 maps + Ch19 scoped threads and Mutex) Count whitespace-separated
/// words across all chunks, case-sensitively. A chunk is any text-like value:
/// `chunk.as_ref()` gives its `&str`. Use one scoped worker per chunk; only
/// that worker reads the chunk's text. Each worker counts into its own map
/// first, then locks the shared total once to merge. Return the totals.
fn word_counts<S: AsRef<str> + Sync>(chunks: &[S]) -> BTreeMap<String, usize> {
    let totals: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
    thread::scope(|scope| {
        for chunk in chunks {
            let totals = &totals;
            scope.spawn(move || {
                // Private map: no lock is needed, and the keys borrow the chunk.
                let mut local: BTreeMap<&str, usize> = BTreeMap::new();
                for word in chunk.as_ref().split_whitespace() {
                    *local.entry(word).or_default() += 1;
                }
                // One short critical section per worker.
                let mut totals = totals.lock().expect("a word counter panicked");
                for (word, n) in local {
                    match totals.get_mut(word) {
                        Some(total) => *total += n,
                        // A String is allocated only for a word that is new.
                        None => {
                            totals.insert(word.to_owned(), n);
                        }
                    }
                }
            });
        }
    });
    totals.into_inner().expect("a word counter panicked")
}

/// C. (Ch14 boxed closures + Ch19 Send) Run every job on its own new thread
/// (never on the caller's thread) and return the results in input order.
fn run_all<T: Send + 'static>(jobs: Vec<Box<dyn FnOnce() -> T + Send>>) -> Vec<T> {
    // A boxed `FnOnce() -> T + Send` is itself a closure that `spawn` accepts.
    let workers: Vec<thread::JoinHandle<T>> = jobs.into_iter().map(thread::spawn).collect();
    workers
        .into_iter()
        .map(|worker| worker.join().expect("job panicked"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::{collections::HashSet, sync::Arc, thread::ThreadId};

    #[test]
    fn a_counts_lines_per_source_in_order() {
        let results = line_counts::<&[u8]>(vec![
            b"one\ntwo\n",
            b"",
            b"no newline",
            &[b'o', b'k', b'\n', 0xff, b'\n'],
            "crab 🦀\r\nline\n".as_bytes(),
        ]);
        let summary: Vec<Result<usize, io::ErrorKind>> = results
            .into_iter()
            .map(|r| r.map_err(|e| e.kind()))
            .collect();
        assert_eq!(
            summary,
            [Ok(2), Ok(0), Ok(1), Err(io::ErrorKind::InvalidData), Ok(2)]
        );
        assert!(line_counts::<Cursor<Vec<u8>>>(vec![]).is_empty());
    }

    #[test]
    fn a_every_source_is_read_on_its_own_thread() {
        /// A reader that notes which thread reads from it.
        struct Spy {
            data: Cursor<Vec<u8>>,
            source: usize,
            seen: Arc<Mutex<Vec<(usize, ThreadId)>>>,
        }
        impl Read for Spy {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                self.seen
                    .lock()
                    .unwrap()
                    .push((self.source, thread::current().id()));
                self.data.read(buf)
            }
        }
        let seen: Arc<Mutex<Vec<(usize, ThreadId)>>> = Arc::default();
        let sources: Vec<Spy> = (0..4)
            .map(|source| Spy {
                data: Cursor::new("line\n".repeat(source).into_bytes()),
                source,
                seen: Arc::clone(&seen),
            })
            .collect();
        let counts: Vec<usize> = line_counts(sources)
            .into_iter()
            .map(|count| count.unwrap())
            .collect();
        assert_eq!(counts, [0, 1, 2, 3]);
        let seen = seen.lock().unwrap();
        assert!(
            seen.iter()
                .all(|(_, thread)| *thread != thread::current().id()),
            "a source was read on the calling thread: move each source into a worker"
        );
        let readers: HashSet<(usize, ThreadId)> = seen.iter().copied().collect();
        assert_eq!(
            readers.len(),
            4,
            "each source must be read by exactly one thread"
        );
        let threads: HashSet<ThreadId> = readers.iter().map(|(_, thread)| *thread).collect();
        assert_eq!(
            threads.len(),
            4,
            "4 sources need 4 different worker threads"
        );
    }

    #[test]
    fn b_word_counts_merge_every_chunk() {
        // The time limit turns a deadlock on the shared map into a failure.
        let counts = within(
            20,
            "word_counts (is a worker waiting for a lock that is never released?)",
            || word_counts(&["the cat", "the  hat\tsat", "", "Cat été été"]),
        );
        let expected: BTreeMap<String, usize> = [
            ("Cat", 1),
            ("cat", 1),
            ("hat", 1),
            ("sat", 1),
            ("the", 2),
            ("été", 2),
        ]
        .into_iter()
        .map(|(w, n)| (w.to_string(), n))
        .collect();
        assert_eq!(counts, expected);
        assert!(word_counts::<&str>(&[]).is_empty());
        assert_eq!(word_counts(&[String::from("owned too")]).len(), 2);
    }

    #[test]
    fn b_many_chunks_lose_no_updates() {
        let counts = within(
            20,
            "word_counts with 64 chunks (is a worker waiting for a lock that is never released?)",
            || word_counts(&vec!["a b a"; 64]),
        );
        assert_eq!(counts["a"], 128);
        assert_eq!(counts["b"], 64);
    }

    #[test]
    fn b_every_chunk_is_read_on_its_own_worker() {
        /// A chunk of text that notes which thread asks for it.
        struct Spy<'a> {
            text: &'a str,
            chunk: usize,
            seen: &'a Mutex<Vec<(usize, ThreadId)>>,
        }
        impl AsRef<str> for Spy<'_> {
            fn as_ref(&self) -> &str {
                self.seen
                    .lock()
                    .unwrap()
                    .push((self.chunk, thread::current().id()));
                self.text
            }
        }
        let seen = Mutex::new(Vec::new());
        let chunks: Vec<Spy> = ["x y", "y z", "z z z"]
            .into_iter()
            .enumerate()
            .map(|(chunk, text)| Spy {
                text,
                chunk,
                seen: &seen,
            })
            .collect();
        let counts = word_counts(&chunks);
        assert_eq!((counts["x"], counts["y"], counts["z"]), (1, 2, 4));
        let seen = seen.into_inner().unwrap();
        assert!(
            seen.iter()
                .all(|(_, thread)| *thread != thread::current().id()),
            "a chunk was read on the calling thread: each chunk belongs to a scoped worker"
        );
        let readers: HashSet<(usize, ThreadId)> = seen.iter().copied().collect();
        assert_eq!(
            readers.len(),
            3,
            "each chunk must be read by exactly one thread"
        );
        let threads: HashSet<ThreadId> = readers.iter().map(|(_, thread)| *thread).collect();
        assert_eq!(threads.len(), 3, "3 chunks need 3 different scoped workers");
    }

    #[test]
    fn c_results_keep_input_order() {
        let jobs: Vec<Box<dyn FnOnce() -> String + Send>> = vec![
            Box::new(|| "first".to_string()),
            Box::new(|| "second".repeat(2)),
            Box::new(String::new),
        ];
        assert_eq!(run_all(jobs), ["first", "secondsecond", ""]);
        assert!(run_all::<u8>(Vec::new()).is_empty());
    }

    #[test]
    fn c_every_job_gets_its_own_thread() {
        let owned = String::from("moved into the job");
        let mut jobs: Vec<Box<dyn FnOnce() -> (thread::ThreadId, usize) + Send>> =
            vec![Box::new(move || (thread::current().id(), owned.len()))];
        for n in 0..5 {
            jobs.push(Box::new(move || (thread::current().id(), n)));
        }
        let results = run_all(jobs);
        let ids: HashSet<_> = results.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids.len(), 6, "each job must run on a different thread");
        assert!(
            !ids.contains(&thread::current().id()),
            "no job may run on the caller's thread"
        );
        assert_eq!(results[0].1, 18);
    }

    /// P. Earlier-chapter recall: predict each value BEFORE running.
    #[test]
    fn p_predictions() {
        // P1 (Ch18). What does read_line return, and what does it store?
        let mut line = String::new();
        let n = Cursor::new("ab\ncd").read_line(&mut line).unwrap();
        let p1: (usize, String) = (3, "ab\n".to_string());
        assert_eq!(p1, (n, line), "P1: read_line keeps the newline");

        // P2 (Ch17). Byte length of "Straße", and its uppercase form.
        let p2: (usize, String) = (7, "STRASSE".to_string());
        assert_eq!(
            p2,
            ("Straße".len(), "Straße".to_uppercase()),
            "P2: UTF-8 length and case mapping"
        );

        // P3 (Ch16). Entry counting into a BTreeMap, then iterating it.
        let mut m = BTreeMap::new();
        for w in "b a b c a b".split(' ') {
            *m.entry(w).or_insert(0) += 1;
        }
        let p3: Vec<(&str, i32)> = vec![("a", 2), ("b", 3), ("c", 1)];
        assert_eq!(
            p3,
            m.into_iter().collect::<Vec<_>>(),
            "P3: BTreeMap iteration order"
        );

        // P4 (Ch15). A reversed, stepped range.
        let p4: Vec<i32> = vec![10, 7, 4, 1];
        assert_eq!(
            p4,
            (1..=10).rev().step_by(3).collect::<Vec<_>>(),
            "P4: rev then step_by"
        );
    }
}
