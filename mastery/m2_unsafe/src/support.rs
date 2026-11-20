//! Support code for the lessons. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`within`] turns a test that would hang into a test that fails with a message.
//! - [`alloc`] audits heap use, so tests can check *how* you manage memory.
//! - [`impls!`](crate::impls) answers "does this type implement this trait?" as a `bool`.
//! - [`gate!`](crate::gate) switches a lesson's tests on once you have designed its types.
//! - [`Verdict`] and [`verdict_matches`] check the Miri verdicts you record in lesson 06.

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// Runs `f` on a helper thread and returns its result, or fails the test with a
/// clear message if it has not finished after `secs` seconds.
///
/// Tests use this around code that would block forever when a lock is never
/// released. `what` names the thing being run and appears in the failure message.
#[track_caller]
pub fn within<R: Send + 'static>(
    secs: u64,
    what: &str,
    f: impl FnOnce() -> R + Send + 'static,
) -> R {
    use std::sync::mpsc::{self, RecvTimeoutError};
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "{what} was still running after {secs} s. It is probably blocked forever: \
             look for a lock that is never released."
        ),
        Err(RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked before finishing; see the panic message above")
        }
    }
}

/// Evaluates to `true` if the type implements the bounds, and to `false` if not.
///
/// ```
/// use m2_unsafe::impls;
/// use std::rc::Rc;
/// assert!(impls!(u8: Send + Sync));
/// assert!(!impls!(Rc<u8>: Send));
/// ```
///
/// The type must be written out in full: inside a generic function, a type
/// parameter `T` always gives `false`, because the answer is decided where the
/// macro is written. How it works: an inherent associated constant is preferred
/// over a trait's constant, but it only exists when its `where` bounds hold.
#[macro_export]
macro_rules! impls {
    ($ty:ty: $($bounds:tt)+) => {{
        #[allow(dead_code)]
        struct Probe<T: ?Sized>(::core::marker::PhantomData<T>);
        #[allow(dead_code)]
        trait Fallback {
            const IMPLS: bool = false;
        }
        impl<T: ?Sized> Fallback for Probe<T> {}
        #[allow(dead_code)]
        impl<T: ?Sized + $($bounds)+> Probe<T> {
            const IMPLS: bool = true;
        }
        <Probe<$ty>>::IMPLS
    }};
}

/// Switches the tests of a lesson in which you design the types yourself.
///
/// The tests name types that do not exist until you write them, so the starter
/// keeps them out of the build: with `off`, the tokens after `=>` are thrown
/// away and one failing test reminds you of the switch. With `on`, the tokens
/// are compiled unchanged.
#[macro_export]
macro_rules! gate {
    (off, $lesson:literal => $($tests:tt)*) => {
        #[cfg(test)]
        mod gate {
            #[test]
            fn tests_are_switched_off() {
                panic!(concat!(
                    "lesson ",
                    $lesson,
                    ": the tests are switched off. Write every item that the contract \
                     names (start with `todo!()` bodies), then change `off` to `on` in \
                     the `gate!` line above the tests."
                ));
            }
        }
    };
    (on, $lesson:literal => $($tests:tt)*) => {
        $($tests)*
    };
}

/// What Miri said about one specimen under one aliasing model (lesson 06).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Miri ran the specimen to the end and printed its result.
    Accepted,
    /// Miri stopped with `error: Undefined Behavior`.
    Rejected,
}

/// Checks one recorded pair of verdicts (Stacked Borrows, Tree Borrows) for a
/// specimen of lesson 06. The expected pairs are stored as checksums, so the
/// only way to learn them is to run Miri.
pub fn verdict_matches(specimen: char, stacked: Verdict, tree: Verdict) -> bool {
    const SUMS: [(char, u32); 8] = [
        ('a', 0x2d14eba9),
        ('b', 0xd801dc02),
        ('c', 0xa18a6e7e),
        ('d', 0x2302c5f8),
        ('e', 0xfe462cd2),
        ('f', 0xa9c27650),
        ('g', 0x4140e043),
        ('h', 0x50bd66f7),
    ];
    SUMS.iter()
        .any(|&(name, sum)| name == specimen && sum == verdict_sum(specimen, stacked, tree))
}

/// FNV-1a over the three inputs; deliberately not readable at a glance.
fn verdict_sum(specimen: char, stacked: Verdict, tree: Verdict) -> u32 {
    let bytes = [
        specimen as u8,
        stacked as u8 + 0x51,
        tree as u8 + 0xa7,
        0x4d,
        0x32,
    ];
    bytes.iter().fold(0x811c_9dc5_u32, |hash, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

pub mod alloc {
    //! A global allocator that audits heap use per thread.
    //!
    //! The test build of this package installs [`Counting`] as the global
    //! allocator. [`audit`] then reports what a closure did with the heap on the
    //! calling thread: how often it allocated, whether it freed everything, and
    //! whether every block was freed with the layout it was allocated with.
    //!
    //! Under Miri the allocator is **not** installed. Miri checks layouts, leaks
    //! and zero-sized requests itself, and more strictly, but it can only do so
    //! for its own allocator. [`counting`] tells a test which of the two is
    //! active; without the allocator every number in an [`Audit`] is zero.
    //!
    //! The optimizer may remove allocations in release builds, so tests assert
    //! upper bounds, never exact non-zero counts.

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    /// How many live blocks and requested layouts one audit can remember.
    const SLOTS: usize = 64;

    #[derive(Clone, Copy)]
    struct Block {
        addr: usize,
        size: usize,
        align: usize,
    }

    const NO_BLOCK: Block = Block {
        addr: 0,
        size: 0,
        align: 0,
    };

    /// What a closure did with the heap. See [`audit`].
    #[derive(Clone, Copy)]
    pub struct Audit {
        /// Calls that obtained or resized memory (`alloc`, `alloc_zeroed`, `realloc`).
        pub allocations: usize,
        /// Blocks allocated minus blocks freed. Zero means that every block
        /// allocated inside the closure was also freed inside it.
        pub live: isize,
        /// Blocks freed or resized with a layout that differs from the layout
        /// they were allocated with. Any value above zero is undefined behaviour.
        pub bad_frees: usize,
        /// Requests for zero bytes. The allocator's contract forbids them.
        pub zero_sized: usize,
        requested: [Block; SLOTS],
    }

    impl Audit {
        const EMPTY: Audit = Audit {
            allocations: 0,
            live: 0,
            bad_frees: 0,
            zero_sized: 0,
            requested: [NO_BLOCK; SLOTS],
        };

        /// True if one of the first 64 requests asked for exactly this layout.
        pub fn requested(&self, layout: Layout) -> bool {
            self.requested
                .iter()
                .take(self.allocations)
                .any(|r| r.size == layout.size() && r.align == layout.align())
        }
    }

    impl std::fmt::Debug for Audit {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Audit")
                .field("allocations", &self.allocations)
                .field("live", &self.live)
                .field("bad_frees", &self.bad_frees)
                .field("zero_sized", &self.zero_sized)
                .finish_non_exhaustive()
        }
    }

    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static AUDIT: Cell<Audit> = const { Cell::new(Audit::EMPTY) };
        /// The blocks that were allocated inside the running audit and are still live.
        static BLOCKS: Cell<[Block; SLOTS]> = const { Cell::new([NO_BLOCK; SLOTS]) };
    }

    /// Runs `update` on this thread's audit if one is running. Never allocates:
    /// the thread-locals are const-initialized and have no destructors.
    fn record(update: impl FnOnce(&mut Audit, &mut [Block; SLOTS])) {
        let _ = ACTIVE.try_with(|active| {
            if active.get() {
                let _ = AUDIT.try_with(|audit| {
                    let _ = BLOCKS.try_with(|blocks| {
                        let (mut a, mut b) = (audit.get(), blocks.get());
                        update(&mut a, &mut b);
                        audit.set(a);
                        blocks.set(b);
                    });
                });
            }
        });
    }

    fn on_alloc(ptr: *mut u8, layout: Layout) {
        record(|audit, blocks| {
            if audit.allocations < SLOTS {
                audit.requested[audit.allocations] = Block {
                    addr: 0,
                    size: layout.size(),
                    align: layout.align(),
                };
            }
            audit.allocations += 1;
            if layout.size() == 0 {
                audit.zero_sized += 1;
            }
            if ptr.is_null() {
                return;
            }
            audit.live += 1;
            if let Some(slot) = blocks.iter_mut().find(|b| b.addr == 0) {
                *slot = Block {
                    addr: ptr.addr(),
                    size: layout.size(),
                    align: layout.align(),
                };
            }
        });
    }

    /// Forgets the block at `ptr` and reports whether `layout` matches the
    /// layout it was allocated with. Blocks from outside the audit are unknown.
    fn on_free(ptr: *mut u8, layout: Layout) {
        record(|audit, blocks| {
            audit.live -= 1;
            if let Some(slot) = blocks.iter_mut().find(|b| b.addr == ptr.addr()) {
                if slot.size != layout.size() || slot.align != layout.align() {
                    audit.bad_frees += 1;
                }
                *slot = NO_BLOCK;
            }
        });
    }

    /// The system allocator is never asked for zero bytes, even when the code
    /// under test asks for them by mistake: the request is counted in
    /// [`Audit::zero_sized`] and then served as one byte.
    fn served(layout: Layout) -> Layout {
        if layout.size() == 0 {
            Layout::from_size_align(1, layout.align()).unwrap_or(layout)
        } else {
            layout
        }
    }

    /// Forwards to the system allocator and records what happens during an [`audit`].
    pub struct Counting;

    // SAFETY: every method forwards to `System` with the layout that `served`
    // computes, and `served` is a pure function of the caller's layout, so a
    // block is always freed or resized with the layout it was obtained with.
    // `System` upholds the rest of the GlobalAlloc contract. Recording touches
    // only const-initialized thread-locals without destructors, so it never
    // allocates and never reenters the allocator.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: `served(layout)` has a non-zero size and a valid alignment.
            let ptr = unsafe { System.alloc(served(layout)) };
            on_alloc(ptr, layout);
            ptr
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            // SAFETY: `served(layout)` has a non-zero size and a valid alignment.
            let ptr = unsafe { System.alloc_zeroed(served(layout)) };
            on_alloc(ptr, layout);
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            on_free(ptr, layout);
            // SAFETY: the caller promises that `ptr` came from this allocator
            // with `layout`, so `System` handed it out for `served(layout)`.
            unsafe { System.dealloc(ptr, served(layout)) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: as in `dealloc`, the block belongs to `System` with
            // `served(layout)`. The new size is at least one byte, and the caller
            // promises that it does not overflow `isize` when rounded up.
            let new_ptr = unsafe { System.realloc(ptr, served(layout), new_size.max(1)) };
            if !new_ptr.is_null() {
                on_free(ptr, layout);
                // `layout.align()` is valid, and the caller checked `new_size`.
                if let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) {
                    on_alloc(new_ptr, new_layout);
                }
            }
            new_ptr
        }
    }

    /// True if [`Counting`] is the global allocator, so that [`audit`] sees
    /// real numbers. False under Miri, where Miri's own checks take over.
    pub fn counting() -> bool {
        cfg!(all(test, not(miri)))
    }

    /// Runs `f` and reports what it did with the heap on this thread.
    /// Audits do not nest: an inner audit restarts the outer one's numbers.
    pub fn audit<R>(f: impl FnOnce() -> R) -> (R, Audit) {
        AUDIT.with(|audit| audit.set(Audit::EMPTY));
        BLOCKS.with(|blocks| blocks.set([NO_BLOCK; SLOTS]));
        let was_active = ACTIVE.with(|active| active.replace(true));
        let result = f();
        ACTIVE.with(|active| active.set(was_active));
        (result, AUDIT.with(Cell::get))
    }

    /// Runs `f` and returns its result with the number of allocations and
    /// reallocations it made on this thread.
    pub fn count<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let (result, audit) = audit(f);
        (result, audit.allocations)
    }
}
