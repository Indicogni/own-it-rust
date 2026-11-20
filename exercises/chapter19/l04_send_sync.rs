//! LESSON 04 — Send and Sync contracts
//! Run: cargo test --test chapter19 l04_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Is Arc<RefCell<T>> automatically thread safe?
//! 2. Is &T Send when T is Sync?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain auto traits, ownership transfer, and why a mutex does not make every inner value Send.
//! COMPILER LAB (examples/ch19_scratch.rs): Write require_send::<Rc<u8>>() and require_sync::<RefCell<u8>>() in scratch; record diagnostics. Repair using Arc and Mutex; do not add unsafe impl.

use std::{
    sync::{Arc, Mutex},
    thread,
};
fn require_send<T: Send>() {}
fn require_sync<T: Sync>() {}
/// A. Demonstrate the appropriate bounds for u32, Arc<Vec<u8>>, Mutex<Vec<u8>>,
/// and &str by calling both bound-checking helpers for each. Return "checked".
fn check_bounds() -> &'static str {
    todo!("A: implement the contract")
}
/// B. Share one allocation between two reader threads. Each reader owns one
/// clone of the Arc and calls `read` on the bytes. Start both readers before
/// you join either. Return the two results (first reader, second reader) and
/// the original Arc, which must still be the same allocation and, once both
/// readers are joined, its only owner.
fn shared_readers(data: Arc<Vec<u8>>, read: fn(&[u8]) -> usize) -> (usize, usize, Arc<Vec<u8>>) {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};
    use std::{sync::Barrier, thread::ThreadId};
    #[test]
    fn a_allowed_types_satisfy_bounds() {
        assert_eq!(check_bounds(), "checked");
    }
    #[test]
    fn b_readers_share_one_allocation() {
        let a = Arc::new(vec![1, 2, 3]);
        let (x, y, returned) = shared_readers(Arc::clone(&a), |bytes| bytes.len());
        assert_eq!((x, y), (3, 3));
        assert!(
            Arc::ptr_eq(&a, &returned),
            "return the Arc you were given, not a new Arc around a copy of the Vec"
        );
        assert_eq!(
            Arc::strong_count(&a),
            2,
            "only the test's Arc and the returned Arc may be alive: a reader's clone was kept"
        );
    }
    #[test]
    fn b_readers_release_their_clones() {
        let (x, y, returned) = shared_readers(Arc::new(Vec::new()), |bytes| bytes.len());
        assert_eq!((x, y), (0, 0));
        assert_eq!(
            Arc::strong_count(&returned),
            1,
            "join the readers so their clones are dropped before returning"
        );
    }
    #[test]
    fn b_two_reader_threads_see_the_same_bytes() {
        // (thread that called `read`, address of the bytes it was given)
        static SEEN: Mutex<Vec<(ThreadId, usize)>> = Mutex::new(Vec::new());
        // Each reader waits here for the other one.
        static GATE: Barrier = Barrier::new(2);
        fn spy(bytes: &[u8]) -> usize {
            GATE.wait();
            SEEN.lock()
                .unwrap()
                .push((thread::current().id(), bytes.as_ptr() as usize));
            bytes.len()
        }
        let data = Arc::new(vec![7u8; 64]);
        let address = data.as_ptr() as usize;
        let (caller, lengths) = within(
            10,
            "shared_readers (each reader waits for the other inside `read`, so \
             start both readers before you join either)",
            move || {
                let (x, y, _data) = shared_readers(data, spy);
                (thread::current().id(), (x, y))
            },
        );
        assert_eq!(lengths, (64, 64));
        let seen = SEEN.lock().unwrap();
        assert_eq!(seen.len(), 2, "call `read` once in each of the two readers");
        assert!(
            seen.iter().all(|(thread, _)| *thread != caller),
            "`read` ran on the calling thread: each reader is its own spawned thread"
        );
        assert_ne!(
            seen[0].0, seen[1].0,
            "both reads ran on one thread: spawn two readers"
        );
        assert!(
            seen.iter().all(|(_, seen_at)| *seen_at == address),
            "a reader was given a copy of the bytes: clone the Arc, not the Vec"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let data = Arc::new(vec![7u8; 3]);

        // P1. Strong count seen inside the worker, which owns one clone.
        let clone = Arc::clone(&data);
        let inside = thread::spawn(move || Arc::strong_count(&clone))
            .join()
            .unwrap();
        // P2. Strong count after the worker has been joined.
        let after = Arc::strong_count(&data);
        let p1: usize = guess();
        let p2: usize = guess();
        assert_eq!(p1, inside, "P1: owners while the worker runs");
        assert_eq!(p2, after, "P2: who dropped the worker's clone?");

        // P3. Arc::try_unwrap returns the Vec only for the last owner. Does the
        // first attempt fail, and does the second (after `other` is dropped) succeed?
        let other = Arc::clone(&data);
        let attempt = Arc::try_unwrap(data);
        let first_failed = attempt.is_err();
        let data = attempt.unwrap_err();
        drop(other);
        let second_ok = Arc::try_unwrap(data).is_ok();
        let p3: (bool, bool) = guess();
        assert_eq!(p3, (first_failed, second_ok), "P3: Arc::try_unwrap");
    }
}
