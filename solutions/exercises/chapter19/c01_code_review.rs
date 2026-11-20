//! CODE REVIEW 01 — A teammate's pull request
//! Run: cargo test --test chapter19 c01_
//! Then: cargo clippy --test chapter19
//!
//! This is the kind of code you will be asked to review at work. It compiles,
//! and one behavior test passes. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every bug, every place a lock is held too long, and every panic
//!    that should have been an error.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep both signatures; the tests depend on them.
//! 4. Run clippy. It reports one warning in this file (a loop over indexes in
//!    `refresh`); your fix should make it disappear.

use std::{collections::BTreeMap, sync::RwLock, thread};

/// Apply `f` to every item using at most `workers` threads (treat 0 as 1),
/// returning the results in input order. With two or more workers and two or
/// more items, the work must really be spread over more than one thread.
/// If `f` panics on any item, however many, return Err("worker panicked")
/// instead of panicking.
fn map_parallel<T, R, F>(items: &[T], workers: usize, f: F) -> Result<Vec<R>, &'static str>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    if items.is_empty() {
        // `chunks(0)` would panic, and there is nothing to spawn anyway.
        return Ok(Vec::new());
    }
    // Rounding up gives at most `workers` chunks, so at most `workers` threads.
    let per_worker = items.len().div_ceil(workers.max(1));
    let f = &f;
    thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(per_worker)
            .map(|chunk| scope.spawn(move || chunk.iter().map(f).collect::<Vec<R>>()))
            .collect();
        // Join EVERY handle before looking at any result. A scoped thread that
        // panicked and was never joined makes `thread::scope` itself panic, so
        // an early `?` here would break the "no panic" promise as soon as two
        // workers fail.
        let joined: Vec<thread::Result<Vec<R>>> =
            handles.into_iter().map(|handle| handle.join()).collect();
        let mut results = Vec::with_capacity(items.len());
        for part in joined {
            // Chunks are joined in spawn order, so the output is in input order.
            results.extend(part.map_err(|_payload| "worker panicked")?);
        }
        Ok(results)
    })
}

/// Fetch a fresh value for every key and make the cache hold exactly those
/// entries (keys not in `keys` are stale and must go). Fetching is slow, so
/// readers of the cache must not be blocked while `fetch` runs.
fn refresh(cache: &RwLock<BTreeMap<String, u32>>, keys: &[&str], fetch: impl Fn(&str) -> u32) {
    // All the slow work happens before the lock is taken.
    let fresh: BTreeMap<String, u32> = keys
        .iter()
        .map(|&key| (key.to_owned(), fetch(key)))
        .collect();
    // One short critical section. Replacing the whole map removes stale keys,
    // and readers see either the old cache or the new one, never a mixture.
    // The data is overwritten completely, so a poisoned lock can be reused.
    *cache.write().unwrap_or_else(|poison| poison.into_inner()) = fresh;
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::within;
    use std::{
        cell::Cell,
        collections::HashSet,
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Condvar, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread::ThreadId,
        time::Duration,
    };

    #[test]
    fn a_results_keep_input_order() {
        let items = [1, 2, 3, 4, 5];
        assert_eq!(
            map_parallel(&items, 2, |x| x * 10),
            Ok(vec![10, 20, 30, 40, 50])
        );
        assert_eq!(map_parallel(&[] as &[u8], 3, |x| *x), Ok(vec![]));
    }

    #[test]
    fn a_empty_and_single_inputs_work() {
        // This test passes before you change anything. It must still pass
        // after your fix: no input means no worker, and one item needs one.
        assert_eq!(map_parallel(&[] as &[u8], 3, |x| *x), Ok(vec![]));
        assert_eq!(map_parallel(&[7], 4, |x| x + 1), Ok(vec![8]));
        assert_eq!(map_parallel(&[7], 0, |x| x + 1), Ok(vec![8]));
    }

    #[test]
    fn a_zero_workers_means_one() {
        assert_eq!(map_parallel(&["a", "bb"], 0, |s| s.len()), Ok(vec![1, 2]));
    }

    #[test]
    fn a_worker_panic_becomes_an_error() {
        let result = map_parallel(&[1, 2, 3], 2, |&x| {
            if x == 2 {
                panic!("fixture: bad item");
            }
            x
        });
        assert_eq!(result, Err("worker panicked"));
    }

    #[test]
    fn a_panics_in_every_worker_are_still_one_error() {
        // Every item panics, so every worker fails. If the function returns
        // while a panicked scoped thread is still unjoined, `thread::scope`
        // panics with "a scoped thread panicked".
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            map_parallel(&[1, 2, 3, 4], 2, |&x: &i32| -> i32 {
                panic!("fixture: item {x} is bad")
            })
        }));
        let result = outcome.expect(
            "map_parallel panicked instead of returning Err: join every worker \
             before you return, also after the first failure",
        );
        assert_eq!(result, Err("worker panicked"));
    }

    #[test]
    fn a_respects_the_worker_limit() {
        let items: Vec<u32> = (0..40).collect();
        let seen: Mutex<HashSet<ThreadId>> = Mutex::new(HashSet::new());
        let out = map_parallel(&items, 3, |x| {
            seen.lock().unwrap().insert(thread::current().id());
            x + 1
        })
        .unwrap();
        assert_eq!(out, (1..41).collect::<Vec<_>>());
        let threads = seen.into_inner().unwrap().len();
        assert!(
            threads <= 3,
            "40 items ran on {threads} threads; spawn at most `workers` threads"
        );
    }

    #[test]
    fn a_work_is_spread_over_more_than_one_thread() {
        let items: Vec<u32> = (0..40).collect();
        let seen: Mutex<HashSet<ThreadId>> = Mutex::new(HashSet::new());
        let another_thread_arrived = Condvar::new();
        let gave_up = AtomicBool::new(false);
        let out = map_parallel(&items, 3, |x| {
            let mut seen = seen.lock().unwrap();
            seen.insert(thread::current().id());
            another_thread_arrived.notify_all();
            // Hold the first thread back until a second one has started on an
            // item. A version that runs everything on one thread gives up
            // after five seconds and fails the assertion below.
            if !gave_up.load(Ordering::Relaxed) {
                let (_seen, wait) = another_thread_arrived
                    .wait_timeout_while(seen, Duration::from_secs(5), |seen| seen.len() < 2)
                    .unwrap();
                if wait.timed_out() {
                    gave_up.store(true, Ordering::Relaxed);
                }
            }
            x + 1
        })
        .unwrap();
        assert_eq!(out, (1..41).collect::<Vec<_>>());
        let threads = seen.into_inner().unwrap().len();
        assert!(
            threads >= 2,
            "40 items with workers = 3 all ran on one thread; split the items \
             between the workers"
        );
    }

    #[test]
    fn b_refresh_replaces_stale_entries() {
        let map = within(
            10,
            "refresh (is a read or write guard still alive when you lock again?)",
            || {
                let cache = RwLock::new(BTreeMap::from([
                    ("old".to_string(), 1),
                    ("a".to_string(), 2),
                ]));
                refresh(&cache, &["a", "b"], |key| key.len() as u32 * 100);
                cache.into_inner().unwrap()
            },
        );
        assert_eq!(
            map,
            BTreeMap::from([("a".to_string(), 100), ("b".to_string(), 100)]),
            "the cache must hold exactly the requested keys with fresh values; \
             \"old\" was not requested and must be gone"
        );
    }

    #[test]
    fn b_readers_are_not_blocked_while_fetching() {
        let (entries, blocked) = within(
            10,
            "refresh (is a read or write guard still alive when you lock again?)",
            || {
                let cache = RwLock::new(BTreeMap::new());
                let blocked = Cell::new(0);
                refresh(&cache, &["x", "y", "z"], |key| {
                    if cache.try_read().is_err() {
                        blocked.set(blocked.get() + 1);
                    }
                    key.len() as u32
                });
                (cache.read().unwrap().len(), blocked.get())
            },
        );
        assert_eq!(entries, 3);
        assert_eq!(
            blocked, 0,
            "the write lock was held during fetch; fetch first, then lock briefly"
        );
    }
}
