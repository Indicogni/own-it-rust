//! LESSON 05 — Mutex invariants and poisoning
//! Run: cargo test --test chapter19 l05_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Can a non-mut Mutex binding still allow mutation inside its guard?
//! 2. Does poisoning prove the protected value is corrupt?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: State the counter invariant. Explain guard lifetimes, panic poisoning, lock order, and why recursive locking can deadlock.
//! COMPILER LAB (examples/ch19_scratch.rs): Create two locks and write down a conflicting lock-order trace without running it. Repair every path to acquire in the same order.

use std::{
    sync::{Arc, Mutex},
    thread,
};
/// A. Spawn `workers` threads that share one counter behind a Mutex. Each
/// worker calls `amount()` `increments` times and adds every result to the
/// counter. Call `amount` while you do NOT hold the lock: the lock protects
/// the addition only. Join all workers; return the final value. Inputs
/// guarantee no usize overflow.
fn count(workers: usize, increments: usize, amount: fn() -> usize) -> usize {
    todo!("A: implement the contract")
}
/// B. Reset an ordinary or poisoned mutex to replacement, then clear poison.
/// This explicit recovery discards the old value; it does not trust its invariant.
fn reset(value: &Mutex<Vec<u8>>, replacement: Vec<u8>) {
    todo!("B: implement the contract")
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{guess, within};
    use std::{cell::Cell, collections::HashSet, sync::Barrier, thread::ThreadId};
    /// Runs task A with a time limit, so a deadlock (for example a guard that
    /// is still held while the workers are joined) fails instead of hanging.
    fn count_within(workers: usize, increments: usize, amount: fn() -> usize) -> usize {
        within(
            20,
            "count (is a guard still alive while you join the workers?)",
            move || count(workers, increments, amount),
        )
    }
    #[test]
    fn a_shared_updates_are_not_lost() {
        assert_eq!(count_within(4, 250, || 1), 1000);
        assert_eq!(count_within(0, 10, || 1), 0);
        assert_eq!(count_within(3, 0, || 1), 0);
        assert_eq!(count_within(8, 10_000, || 1), 80_000);
        assert_eq!(count_within(2, 5, || 7), 70);
    }
    #[test]
    fn a_every_worker_is_its_own_thread() {
        static SEEN: Mutex<Vec<ThreadId>> = Mutex::new(Vec::new());
        fn spy() -> usize {
            SEEN.lock().unwrap().push(thread::current().id());
            1
        }
        let (caller, total) = within(
            20,
            "count (is a guard still alive while you join the workers?)",
            || (thread::current().id(), count(5, 3, spy)),
        );
        assert_eq!(total, 15);
        let seen = SEEN.lock().unwrap();
        assert_eq!(
            seen.len(),
            15,
            "5 workers x 3 increments means 15 calls to `amount`"
        );
        assert!(
            !seen.contains(&caller),
            "`amount` ran on the calling thread: the workers do the counting"
        );
        let distinct: HashSet<&ThreadId> = seen.iter().collect();
        assert_eq!(distinct.len(), 5, "5 workers must be 5 different threads");
    }
    #[test]
    fn a_lock_is_free_while_a_worker_computes() {
        // The first call to `amount` on each worker waits until all three
        // workers are inside `amount`. That can only happen if nobody holds
        // the lock there, and if the workers run at the same time.
        static GATE: Barrier = Barrier::new(3);
        fn gated() -> usize {
            thread_local! {
                static WAITED: Cell<bool> = const { Cell::new(false) };
            }
            if !WAITED.replace(true) {
                GATE.wait();
            }
            1
        }
        let total = within(
            10,
            "count (3 workers wait for each other inside `amount`; that needs all \
             workers spawned before the first join, and no lock held while \
             `amount` runs)",
            || count(3, 4, gated),
        );
        assert_eq!(total, 12);
    }
    #[test]
    fn b_recovery_reestablishes_an_invariant() {
        let v = Arc::new(Mutex::new(vec![1]));
        let copy = Arc::clone(&v);
        let panicker = thread::spawn(move || {
            let mut g = copy.lock().unwrap();
            g.push(2);
            panic!("poison fixture")
        });
        assert!(panicker.join().is_err());
        assert!(v.is_poisoned());
        reset(&v, vec![9]);
        assert!(
            !v.is_poisoned(),
            "reset must also remove the poison mark: see Mutex::clear_poison"
        );
        assert_eq!(
            *v.lock().unwrap(),
            [9],
            "reset must discard the old value, including what the panicking worker pushed"
        );
        reset(&v, vec![]);
        assert!(v.lock().unwrap().is_empty());
    }
    #[test]
    fn b_reset_works_on_a_healthy_mutex() {
        let m = Mutex::new(vec![1, 2, 3]);
        reset(&m, vec![4]);
        assert!(!m.is_poisoned());
        assert_eq!(*m.lock().unwrap(), [4]);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Is the lock free right after this statement? While `guard` is alive?
        let m = Mutex::new(vec![1, 2, 3]);
        let len = m.lock().unwrap().len();
        let free_after_statement = m.try_lock().is_ok();
        let guard = m.lock().unwrap();
        let free_while_guarded = m.try_lock().is_ok();
        drop(guard);
        let p1: (bool, bool) = guess();
        assert_eq!(
            p1,
            (free_after_statement, free_while_guarded),
            "P1: when a guard is dropped"
        );

        // P2. A worker pushes 4, then panics while holding the guard.
        // Is the mutex poisoned, and what length does recovery reveal?
        let shared = Arc::new(Mutex::new(vec![1, 2, 3]));
        let copy = Arc::clone(&shared);
        let _ = thread::spawn(move || {
            copy.lock().unwrap().push(4);
            let _guard = copy.lock().unwrap();
            panic!("fixture: panic while locked");
        })
        .join();
        let poisoned = shared.is_poisoned();
        let len = shared.lock().unwrap_or_else(|e| e.into_inner()).len();
        let p2: (bool, usize) = guess();
        assert_eq!(p2, (poisoned, len), "P2: poisoning does not roll back");

        // P3. This worker panics while it holds NO guard. Poisoned?
        let calm = Arc::new(Mutex::new(0));
        let copy = Arc::clone(&calm);
        let _ = thread::spawn(move || {
            *copy.lock().unwrap() += 1;
            panic!("fixture: panic after unlocking");
        })
        .join();
        let p3: bool = guess();
        assert_eq!(p3, calm.is_poisoned(), "P3: only a panicking guard poisons");

        // P4. The guard here is a temporary created in the condition of an
        // `if let`, or in the scrutinee of a `match`. Is the lock free
        // (a) inside the `if let` body, (b) inside its `else`, (c) inside the
        // match arm? This course uses edition 2024.
        let full = Mutex::new(Some(1));
        let in_body = if let Some(_n) = *full.lock().unwrap() {
            full.try_lock().is_ok()
        } else {
            unreachable!()
        };
        let empty = Mutex::new(None::<i32>);
        let in_else = if let Some(_n) = *empty.lock().unwrap() {
            unreachable!()
        } else {
            empty.try_lock().is_ok()
        };
        let in_arm = match *empty.lock().unwrap() {
            Some(_n) => unreachable!(),
            None => empty.try_lock().is_ok(),
        };
        let p4: (bool, bool, bool) = guess();
        assert_eq!(
            p4,
            (in_body, in_else, in_arm),
            "P4: how long a guard in a condition lives"
        );
    }
}
