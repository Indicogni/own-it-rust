//! LESSON 07 — A lock-free stack, and the ABA problem
//! Run: cargo test l07_
//! Model: cargo model l07_
//! Read first: "Lesson 07" in CONCEPTS.md (same package).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `pop` reads `head`, then reads the `next` field of that node, then does
//!    a compare-exchange on `head`. What can other threads do to that node
//!    between the first step and the last?
//! 2. A compare-exchange compares pointer values. What does it conclude when
//!    the old node was freed and a new node was allocated at the same address?
//!
//! IMPLEMENT the contract below. You design the node type and the fields.
//! Import from `crate::sync`, never from `std`. Keep the supplied tests and
//! add one edge case.
//! EXPLAIN afterward: State the rule that decides when your stack frees a
//! node, and use it twice: once to show that `pop` never reads freed memory,
//! and once to show that the compare-exchange in `pop` cannot succeed by
//! mistake (no ABA). Then say what your rule costs.
//!
//! MODEL LAB (optional): Change the compare-exchange in `push` to
//! `Relaxed`/`Relaxed` and run `cargo model l07_`. Then restore it, and make
//! `pop` free each node at once with `Box::from_raw`. Run the normal tests,
//! then `cargo +nightly-2026-11-20 miri test l07_`. Which tool reports the mistake, and
//! what would it take for loom to see it?

use crate::sync::atomic::{AtomicPtr, Ordering};
use std::mem::MaybeUninit;
use std::ptr;

// CONTRACT
//
// `Stack<T>` is a last-in, first-out collection that many threads use through
// `&Stack<T>` at the same time, without any lock.
//
// 1. `push` and `pop` are lock-free: they never block and never wait for
//    another thread. A thread retries only if another thread's operation
//    succeeded in the meantime. Use an `AtomicPtr` and compare-exchange loops.
// 2. `pop` returns the most recently pushed value that has not been popped,
//    or `None` if the stack is empty. No value is lost and none is returned
//    twice, under any interleaving.
// 3. Memory rule for this lesson: **a node is freed only in `Drop`**. `pop`
//    must not free the node it removed, and `push` must not reuse it. (The
//    concepts text explains why, and what production code does instead.)
//    So `pop` frees nothing, and `push` allocates exactly one node.
// 4. `Drop` frees every node that was ever allocated, and drops every value
//    that is still in the stack exactly once.
// 5. `Stack<T>` is `Send` and `Sync` exactly when `T: Send`.
// 6. Every atomic operation uses the weakest ordering that your argument
//    needs. The model tests reject orderings that are too weak.

/// One heap block per pushed value.
struct Node<T> {
    /// Initialised from `push` until the one `pop` that unlinks this node
    /// moves the value out.
    value: MaybeUninit<T>,
    /// The node below this one. Written before the node is published, and
    /// never changed while the stack is shared.
    next: *mut Node<T>,
    /// Link in the `retired` list. Written only by the thread that unlinked
    /// this node, before it publishes the node there.
    next_retired: *mut Node<T>,
}

/// A Treiber stack that frees its nodes only when it is dropped.
///
/// Invariants:
/// - Every node reachable from `head` or `retired` was allocated by `push`
///   with `Box` and stays allocated until `Drop`. So a node pointer that was
///   read from `head` can be dereferenced for as long as `&self` lives.
/// - A node enters the `head` list once and leaves it once. It never comes
///   back, so "`head` still holds the pointer I read" means "nothing was
///   pushed or popped in between".
pub struct Stack<T> {
    head: AtomicPtr<Node<T>>,
    /// Nodes that `pop` unlinked. Their values are gone; only `Drop` reads
    /// this list.
    retired: AtomicPtr<Node<T>>,
}

// These two impls restrict as much as they permit. `AtomicPtr<_>` is `Send`
// and `Sync` for every pointee, so without them the compiler would make
// `Stack<Rc<u8>>` both. An explicit impl replaces the automatic one.
//
// SAFETY: the stack owns its values and hands each one to exactly one thread
// by value. That is a move between threads, so `T: Send` is required.
unsafe impl<T: Send> Send for Stack<T> {}
// SAFETY: through `&Stack<T>` a thread can only push a `T` or pop one by
// value; no `&T` is ever shared, so `T: Sync` is not needed. The node fields
// are synchronized as described on `push`, `pop` and `retire`.
unsafe impl<T: Send> Sync for Stack<T> {}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack {
            head: AtomicPtr::new(ptr::null_mut()),
            retired: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Puts `value` on top of the stack.
    pub fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(Node {
            value: MaybeUninit::new(value),
            next: ptr::null_mut(),
            next_retired: ptr::null_mut(),
        }));
        // Relaxed: this thread does not look inside the node that `head`
        // points to; it only stores the pointer.
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            // SAFETY: `node` came from `Box::into_raw` above and is not
            // published yet, so this thread has exclusive access to it.
            unsafe { (*node).next = head };
            // Release on success: the writes to `*node` above happen before
            // any Acquire load that reads this pointer from `head`.
            match self
                .head
                .compare_exchange_weak(head, node, Ordering::Release, Ordering::Relaxed)
            {
                Ok(_) => return,
                Err(now) => head = now,
            }
        }
    }

    /// Takes the top value off the stack.
    pub fn pop(&self) -> Option<T> {
        // Acquire: pairs with the Release in the `push` that published this
        // node, so its fields are initialised from this thread's point of
        // view. (Every change of `head` is a compare-exchange, so the load
        // also synchronizes with every earlier `push`: see "release
        // sequence" in the concepts text.)
        let mut head = self.head.load(Ordering::Acquire);
        loop {
            if head.is_null() {
                return None;
            }
            // SAFETY: `head` was read from `self.head`, so the node was
            // published and, by the first invariant, is still allocated.
            // Nobody writes `next` after publication, so this read does not
            // race.
            let next = unsafe { (*head).next };
            // On success, Relaxed: nothing new is published; `next` was
            // already part of the stack. On failure, Acquire: the new value
            // is a node pointer that the next iteration dereferences.
            //
            // No ABA: if `head` still holds the pointer we read, the node
            // never left the stack in between (second invariant), so `next`
            // is still the node below it.
            match self
                .head
                .compare_exchange_weak(head, next, Ordering::Relaxed, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(now) => head = now,
            }
        }
        // SAFETY: this thread's compare-exchange unlinked the node, and a
        // node is unlinked only once, so this is the only read of the value.
        // It is initialised: `push` wrote it before publishing the node.
        let value = unsafe { (&raw const (*head).value).cast::<T>().read() };
        self.retire(head);
        Some(value)
    }

    /// Is the stack empty at this moment? Another thread may change that
    /// before the caller looks at the answer.
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed).is_null()
    }

    /// Keeps an unlinked node for `Drop` instead of freeing it now. Another
    /// `pop` may still be reading its `next` field.
    fn retire(&self, node: *mut Node<T>) {
        // Relaxed everywhere: no thread looks inside the retired nodes until
        // `Drop`, and `Drop` has exclusive access, which orders it after
        // everything that happened through `&self`.
        let mut top = self.retired.load(Ordering::Relaxed);
        loop {
            // SAFETY: the node is allocated (first invariant). Only the
            // thread that unlinked a node writes its `next_retired`, and
            // other threads read only its `next` field.
            unsafe { (*node).next_retired = top };
            match self.retired.compare_exchange_weak(
                top,
                node,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(now) => top = now,
            }
        }
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        // `&mut self`: no other thread uses the stack, and whatever gave this
        // thread exclusive access ordered all earlier operations before this
        // point. Relaxed loads therefore see the final lists.
        let mut live = self.head.load(Ordering::Relaxed);
        while !live.is_null() {
            // SAFETY: every node in the list came from `Box::into_raw` in
            // `push` and is freed nowhere else. Each node is in exactly one
            // of the two lists, so it is freed once.
            let node = unsafe { Box::from_raw(live) };
            live = node.next;
            // SAFETY: a node in the `head` list was never popped, so its
            // value is still initialised.
            drop(unsafe { node.value.assume_init() });
        }
        let mut retired = self.retired.load(Ordering::Relaxed);
        while !retired.is_null() {
            // SAFETY: as above. The value of a retired node was moved out by
            // `pop`; `MaybeUninit` does not drop it again.
            let node = unsafe { Box::from_raw(retired) };
            retired = node.next_retired;
        }
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;
    use crate::implements;
    use crate::support::{
        DropCount,
        alloc::{count, frees},
        guess, within,
    };
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::Arc;

    #[test]
    fn a_last_in_first_out() {
        let stack = Stack::new();
        assert!(stack.is_empty());
        assert_eq!(
            stack.pop(),
            None::<&str>,
            "an empty stack has nothing to pop"
        );
        stack.push("a");
        stack.push("b");
        assert!(!stack.is_empty());
        assert_eq!(stack.pop(), Some("b"));
        stack.push("c");
        assert_eq!(
            [stack.pop(), stack.pop(), stack.pop()],
            [Some("c"), Some("a"), None],
            "values must come out in the reverse of the order in which they went in"
        );
        assert!(stack.is_empty());
    }

    #[test]
    fn b_one_node_per_push_and_no_free_before_drop() {
        let stack = Stack::new();
        let ((), allocations) = count(|| stack.push(std::hint::black_box(1_u64)));
        assert!(
            allocations <= 1,
            "`push` made {allocations} heap allocations; it needs one node"
        );
        stack.push(2);
        stack.push(3);
        let (popped, freed) = frees(|| (stack.pop(), stack.pop()));
        assert_eq!(popped, (Some(3), Some(2)));
        assert_eq!(
            freed, 0,
            "`pop` freed {freed} heap block(s). Another thread may still be reading \
             the node that `pop` removed, so in this lesson nodes are freed only in `Drop`"
        );
        let ((), allocations) = count(|| {
            stack.pop();
        });
        assert_eq!(allocations, 0, "`pop` must not allocate");
        // All three nodes are removed now. The next `push` must still
        // allocate: a removed node must not go back into the stack.
        let ((), allocations) = count(|| stack.push(std::hint::black_box(4)));
        assert_eq!(
            allocations, 1,
            "`push` after three `pop`s made {allocations} heap allocation(s), not 1. \
             A removed node must not be used again while the stack is shared: another \
             thread may still hold a pointer to it, and that is how ABA starts"
        );
        let ((), freed) = frees(|| drop(stack));
        assert!(
            freed >= 4,
            "dropping the stack freed {freed} heap block(s), but 4 nodes were allocated: \
             `Drop` must free the node that is still in the stack and the removed ones"
        );
    }

    #[test]
    fn c_values_are_dropped_exactly_once() {
        let drops = DropCount::new();
        let stack = Stack::new();
        for id in 0..5 {
            stack.push(drops.value(id));
        }
        let top = stack.pop().unwrap();
        assert_eq!(top.id, 4);
        assert_eq!(
            drops.get(),
            0,
            "a popped value belongs to the caller and is alive"
        );
        drop(top);
        assert_eq!(drops.get(), 1);
        drop(stack.pop());
        assert_eq!(drops.get(), 2);
        drop(stack);
        assert_eq!(
            drops.get(),
            5,
            "dropping the stack must drop the 3 values still in it, and must not \
             drop the 2 popped values a second time"
        );
    }

    #[test]
    fn d_send_and_sync_for_send_values() {
        let _ = Stack::<u8>::new();
        assert!(implements!(Stack<u32>: Send + Sync));
        assert!(
            implements!(Stack<Cell<u32>>: Sync),
            "the stack never shares `&T`; it moves values, so `T: Send` is enough"
        );
        assert!(!implements!(Stack<Rc<u32>>: Send));
        assert!(
            !implements!(Stack<Rc<u32>>: Sync),
            "a value pushed on one thread is popped on another, so `T` must be `Send`"
        );
    }

    #[test]
    fn e_threads_lose_and_repeat_nothing() {
        let per_thread: usize = if cfg!(miri) { 20 } else { 5_000 };
        let (popped, drops) = within(60, "the stack stress test", move || {
            let drops = DropCount::new();
            let stack = Arc::new(Stack::new());
            let pushers: Vec<_> = (0..4)
                .map(|thread| {
                    let stack = Arc::clone(&stack);
                    std::thread::spawn(move || {
                        for n in 0..per_thread {
                            stack.push(drops.value(thread * per_thread + n));
                        }
                    })
                })
                .collect();
            let poppers: Vec<_> = (0..4)
                .map(|_| {
                    let stack = Arc::clone(&stack);
                    std::thread::spawn(move || {
                        let mut seen = Vec::new();
                        for _ in 0..per_thread {
                            if let Some(value) = stack.pop() {
                                seen.push(value.id);
                            }
                        }
                        seen
                    })
                })
                .collect();
            for pusher in pushers {
                pusher.join().unwrap();
            }
            let mut popped: Vec<usize> = poppers
                .into_iter()
                .flat_map(|popper| popper.join().unwrap())
                .collect();
            while let Some(value) = stack.pop() {
                popped.push(value.id);
            }
            drop(stack);
            (popped, drops.get())
        });
        let mut sorted = popped.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (0..4 * per_thread).collect::<Vec<_>>(),
            "every pushed value must be popped exactly once"
        );
        assert_eq!(
            drops,
            4 * per_thread,
            "every value must be dropped exactly once"
        );
    }

    /// A stack of slot numbers, as a lock-free free list would keep it:
    /// `next[i]` is the slot below slot `i`, and `head` is the top slot.
    /// `NIL` marks the bottom.
    const NIL: usize = usize::MAX;

    struct Slots {
        head: std::sync::atomic::AtomicUsize,
        next: [usize; 3],
    }

    impl Slots {
        /// top -> 0 -> 1 -> 2 -> NIL
        fn full() -> Self {
            Slots {
                head: std::sync::atomic::AtomicUsize::new(0),
                next: [1, 2, NIL],
            }
        }

        fn pop(&mut self) -> usize {
            let top = *self.head.get_mut();
            *self.head.get_mut() = self.next[top];
            top
        }

        fn push(&mut self, slot: usize) {
            self.next[slot] = *self.head.get_mut();
            *self.head.get_mut() = slot;
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    ///
    /// The code below replays, on one thread, an interleaving of two threads
    /// that use the `Slots` stack above. Slots are reused, as nodes would be
    /// if `pop` freed them and `push` got the same address again.
    #[test]
    fn p_predictions() {
        use std::sync::atomic::Ordering::SeqCst;

        let mut slots = Slots::full();
        // Thread A starts a pop: it reads `head` and the `next` of that slot.
        // Then it is paused, just before its compare-exchange.
        let a_head = slots.head.load(SeqCst);
        let a_next = slots.next[a_head];
        // Thread B runs: it pops two slots and pushes the first one back.
        let first = slots.pop();
        let second = slots.pop();
        slots.push(first);
        // Thread A continues with the values it read before the pause.
        let exchange = slots.head.compare_exchange(a_head, a_next, SeqCst, SeqCst);

        // P1. Does thread A's compare-exchange succeed?
        let p1: bool = true;
        assert_eq!(
            p1,
            exchange.is_ok(),
            "P1: does the stale compare-exchange succeed?"
        );

        // P2. Which slot is on top now, and which slot does thread B still hold?
        let p2: (usize, usize) = (1, 1);
        assert_eq!(
            p2,
            (slots.head.load(SeqCst), second),
            "P2: (the slot that `head` names now, the slot that thread B holds)"
        );

        // P3. The same replay with a version number next to the slot: every
        // successful pop adds 1 to the version, and the compare-exchange
        // compares both. Does thread A's compare-exchange succeed now?
        fn pack(version: usize, slot: usize) -> usize {
            version << 8 | slot
        }
        let head = std::sync::atomic::AtomicUsize::new(pack(0, 0));
        let a_seen = head.load(SeqCst); // thread A: version 0, slot 0
        let a_new = pack(1, 1); // thread A wants: version 1, slot 1
        head.store(pack(1, 1), SeqCst); // thread B pops slot 0
        head.store(pack(2, 2), SeqCst); // thread B pops slot 1
        head.store(pack(2, 0), SeqCst); // thread B pushes slot 0 back
        let versioned = head.compare_exchange(a_seen, a_new, SeqCst, SeqCst);
        let p3: bool = false;
        assert_eq!(
            p3,
            versioned.is_ok(),
            "P3: does the stale compare-exchange succeed when a version is compared too?"
        );
    }
}

#[cfg(all(test, loom))]
mod model {
    use super::*;
    use crate::support::{DropCount, Tracked, model, model_bounded};
    use crate::sync::{Arc, thread};

    #[test]
    fn a_push_on_one_thread_pop_on_another() {
        model(|| {
            let stack = Arc::new(Stack::new());
            let pusher = {
                let stack = Arc::clone(&stack);
                thread::spawn(move || stack.push(Tracked::new(7_u32)))
            };
            // If the value arrives, reading it must be ordered after the
            // write in the pushing thread.
            if let Some(value) = stack.pop() {
                assert_eq!(value.get(), 7);
            }
            pusher.join().unwrap();
        });
    }

    #[test]
    fn b_two_pushers_then_pop_everything() {
        model(|| {
            let drops = DropCount::new();
            let stack = Arc::new(Stack::new());
            let other = {
                let stack = Arc::clone(&stack);
                thread::spawn(move || stack.push(Tracked::counted(1_u32, &drops)))
            };
            stack.push(Tracked::counted(2_u32, &drops));
            other.join().unwrap();
            let mut seen = vec![
                stack.pop().expect("two values were pushed").get(),
                stack.pop().expect("two values were pushed").get(),
            ];
            seen.sort_unstable();
            assert_eq!(seen, [1, 2], "a push was lost");
            assert!(stack.pop().is_none());
            drop(stack);
            assert_eq!(drops.get(), 2, "every value must be dropped exactly once");
        });
    }

    #[test]
    fn c_two_poppers_never_get_the_same_value() {
        model(|| {
            let drops = DropCount::new();
            let stack = Arc::new(Stack::new());
            stack.push(Tracked::counted(1_u32, &drops));
            stack.push(Tracked::counted(2_u32, &drops));
            let other = {
                let stack = Arc::clone(&stack);
                thread::spawn(move || stack.pop().map(|value| value.get()))
            };
            let mine = stack.pop().map(|value| value.get());
            let theirs = other.join().unwrap();
            let mut seen = vec![mine.expect("two values"), theirs.expect("two values")];
            seen.sort_unstable();
            assert_eq!(
                seen,
                [1, 2],
                "two `pop` calls returned the same value, or lost one"
            );
            assert!(stack.pop().is_none());
            drop(stack);
            assert_eq!(drops.get(), 2);
        });
    }

    #[test]
    fn d_push_and_pop_mixed() {
        model_bounded(2, || {
            let drops = DropCount::new();
            let stack = Arc::new(Stack::new());
            stack.push(Tracked::counted(0_u32, &drops));
            let workers: Vec<_> = (1..=2_u32)
                .map(|n| {
                    let stack = Arc::clone(&stack);
                    thread::spawn(move || {
                        stack.push(Tracked::counted(n, &drops));
                        stack.pop().map(|value| value.get())
                    })
                })
                .collect();
            let mut seen: Vec<u32> = workers
                .into_iter()
                .map(|worker| worker.join().unwrap().expect("the stack held a value"))
                .collect();
            while let Some(value) = stack.pop() {
                seen.push(value.get());
            }
            seen.sort_unstable();
            assert_eq!(seen, [0, 1, 2], "a value was lost or returned twice");
            drop(stack);
            assert_eq!(drops.get(), 3);
        });
    }
}
