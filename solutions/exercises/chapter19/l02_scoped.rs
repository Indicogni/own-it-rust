//! LESSON 02 — Borrowed parallel work
//! Run: cargo test --test chapter19 l02_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider:
//! 1. Does Arc clone copy the allocation contents?
//! 2. Can scoped threads mutate disjoint parts of a slice?
//!
//! IMPLEMENT the tasks; keep the tests.
//! EXPLAIN: Explain the relationship between scope lifetime, disjoint borrows, and joining.
//! COMPILER LAB (examples/ch19_scratch.rs): Try capturing one mutable slice twice in scoped closures; repair with split_at_mut. Compare Arc<Vec<_>> for unscoped workers.

use std::thread;
/// A. Sum a borrowed i32 slice in two scoped workers. Split at len/2 and give
/// each half to one worker. A worker calls `widen` on each of its elements and
/// adds up the i64 results; the calling thread only adds the two partial sums.
/// Empty inputs are valid.
fn scoped_sum(values: &[i32], widen: impl Fn(i32) -> i64 + Sync) -> i64 {
    let (left, right) = values.split_at(values.len() / 2);
    // Both workers only need `&widen`. Taking the reference here lets the
    // `move` closures below move the half and copy the reference.
    let widen = &widen;
    thread::scope(|scope| {
        let left = scope.spawn(move || left.iter().map(|&v| widen(v)).sum::<i64>());
        let right = scope.spawn(move || right.iter().map(|&v| widen(v)).sum::<i64>());
        // Both workers are already running; joining only collects the results.
        left.join().expect("left worker panicked") + right.join().expect("right worker panicked")
    })
}
/// B. Apply `update` to every element in two scoped workers (split at len/2),
/// without cloning the data and without a mutex. All calls to `update` happen
/// on the two workers.
fn scoped_update(values: &mut [i32], update: impl Fn(&mut i32) + Sync) {
    let mid = values.len() / 2;
    // Two `&mut` halves that cannot overlap, so each worker may own one.
    let (left, right) = values.split_at_mut(mid);
    let update = &update;
    thread::scope(|scope| {
        scope.spawn(move || left.iter_mut().for_each(update));
        scope.spawn(move || right.iter_mut().for_each(update));
        // No explicit join: the scope joins both workers before it returns.
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use practice::{alloc::count, within};
    use std::{
        sync::{
            Arc, Barrier, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        thread::ThreadId,
    };
    #[test]
    fn a_scoped_workers_borrow_input() {
        assert_eq!(scoped_sum(&[i32::MAX, i32::MAX, -1], i64::from), 4294967293);
        assert_eq!(scoped_sum(&[i32::MIN, i32::MIN], i64::from), -4294967296);
        assert_eq!(scoped_sum(&[], i64::from), 0);
        assert_eq!(scoped_sum(&[-5], i64::from), -5);
        assert_eq!(scoped_sum(&[1, 2, 3], |v| i64::from(v) * 10), 60);
    }
    #[test]
    fn a_each_half_is_summed_on_its_own_worker() {
        let seen: Mutex<Vec<(i32, ThreadId)>> = Mutex::new(Vec::new());
        let values: Vec<i32> = (0..8).collect();
        let sum = scoped_sum(&values, |v| {
            seen.lock().unwrap().push((v, thread::current().id()));
            i64::from(v)
        });
        assert_eq!(sum, 28);
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.len(), 8, "call `widen` exactly once per element");
        let thread_of = |value: i32| seen.iter().find(|(v, _)| *v == value).unwrap().1;
        let (left, right) = (thread_of(0), thread_of(4));
        assert!(
            left != thread::current().id() && right != thread::current().id(),
            "`widen` ran on the calling thread: both halves belong to scoped workers"
        );
        assert_ne!(
            left, right,
            "both halves ran on one thread: spawn one scoped worker per half"
        );
        for (value, thread) in seen {
            let expected = if value < 4 { left } else { right };
            assert_eq!(
                thread, expected,
                "element {value} was handled by the wrong worker: split at len / 2"
            );
        }
    }
    #[test]
    fn a_both_halves_run_at_the_same_time() {
        let sum = within(
            10,
            "scoped_sum (the first element of each half waits for the other half's \
             worker, so spawn both workers before you join either)",
            || {
                let gate = Barrier::new(2);
                scoped_sum(&[1, 2, 3, 4], |v| {
                    if v == 1 || v == 3 {
                        gate.wait();
                    }
                    i64::from(v)
                })
            },
        );
        assert_eq!(sum, 10);
    }
    #[test]
    fn b_scoped_workers_mutate_disjoint_halves() {
        let mut a = [1, 2, 3, 4, 5];
        scoped_update(&mut a, |v| *v -= 2);
        assert_eq!(a, [-1, 0, 1, 2, 3]);
        let mut one = [7];
        scoped_update(&mut one, |v| *v += 1);
        assert_eq!(one, [8]);
        scoped_update(&mut [], |v| *v += 7);
    }
    #[test]
    fn b_scoped_update_edits_in_place() {
        let mut v: Vec<i32> = (0..1000).collect();
        let p = v.as_ptr();
        scoped_update(&mut v, |v| *v += 1);
        assert_eq!(
            v.as_ptr(),
            p,
            "the Vec's buffer changed: update the elements where they are, do not build a new Vec"
        );
        assert!(v.iter().copied().eq(1..1001));
    }
    #[test]
    fn b_each_half_is_updated_on_its_own_worker() {
        let (values, seen) = within(
            10,
            "scoped_update (the first element of each half waits for the other \
             half's worker, so spawn both workers before the scope ends)",
            || {
                let gate = Barrier::new(2);
                let seen: Mutex<Vec<(i32, ThreadId)>> = Mutex::new(Vec::new());
                let mut values = [10, 11, 12, 13, 14, 15];
                scoped_update(&mut values, |v| {
                    if *v == 10 || *v == 13 {
                        gate.wait();
                    }
                    seen.lock().unwrap().push((*v, thread::current().id()));
                    *v *= 2;
                });
                let caller = thread::current().id();
                let seen = seen.into_inner().unwrap();
                assert!(
                    seen.iter().all(|(_, thread)| *thread != caller),
                    "`update` ran on the calling thread: both halves belong to scoped workers"
                );
                (values, seen)
            },
        );
        assert_eq!(values, [20, 22, 24, 26, 28, 30]);
        let thread_of = |value: i32| seen.iter().find(|(v, _)| *v == value).unwrap().1;
        assert_ne!(
            thread_of(10),
            thread_of(13),
            "both halves ran on one thread: spawn one scoped worker per half"
        );
        for (value, thread) in &seen {
            let expected = thread_of(if *value < 13 { 10 } else { 13 });
            assert_eq!(
                *thread, expected,
                "element {value} was handled by the wrong worker: split at len / 2"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Allocations made by Arc::clone, and do both handles point at one Vec?
        let a = Arc::new(vec![1u8; 1024]);
        let (b, allocations) = count(|| Arc::clone(&a));
        let p1: (usize, bool) = (0, true);
        assert_eq!(p1, (allocations, Arc::ptr_eq(&a, &b)), "P1: cloning an Arc");

        // P2. Lengths of the two halves from split_at_mut(len / 2).
        let mut v = [10, 20, 30, 40, 50];
        let mid = v.len() / 2;
        let (left, right) = v.split_at_mut(mid);
        let p2: (usize, usize) = (2, 3);
        assert_eq!(p2, (left.len(), right.len()), "P2: where the split lands");

        // P3. Nobody calls join here. What does the counter read after the scope?
        let hits = AtomicUsize::new(0);
        thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| hits.fetch_add(1, Ordering::Relaxed));
            }
        });
        let p3: usize = 3;
        assert_eq!(p3, hits.into_inner(), "P3: a scope joins its threads");
    }
}
