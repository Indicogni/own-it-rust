//! Sharing mutable state between tasks and interrupt handlers.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::cpu::IrqGuard;

/// A lock for a single-core kernel: the critical section runs with IRQs
/// masked.
///
/// On one core, the only way for other code to run in the middle of a
/// critical section is an interrupt (which may also switch tasks). With
/// IRQs masked that cannot happen, so the closure has exclusive access.
pub struct IrqLock<T> {
    /// Detects a closure that locks the same lock again, which would create
    /// two `&mut T` to the same value.
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: `lock` hands out `&mut T` only while IRQs are masked and the
// `locked` flag is held, so no two contexts (tasks or interrupt handlers)
// ever access the value at the same time. This kernel runs on one core; on
// several cores this type would also need a spin lock. `T: Send` because the
// value is used from whichever task or handler takes the lock.
unsafe impl<T: Send> Sync for IrqLock<T> {}

impl<T> IrqLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Runs `f` with exclusive access to the value.
    ///
    /// The closure must not switch tasks and must be short: interrupts are
    /// delayed for as long as it runs.
    pub fn lock<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let _irq = IrqGuard::new();
        assert!(
            !self.locked.swap(true, Ordering::Relaxed),
            "IrqLock locked twice by the same code path"
        );
        // SAFETY: IRQs are masked, and IRQ is the only kind of interrupt
        // this kernel ever unmasks, so nothing can interrupt this code. The
        // flag check above proves that no enclosing `lock` call on this
        // lock is active. This is therefore the only reference to the value.
        let result = f(unsafe { &mut *self.value.get() });
        self.locked.store(false, Ordering::Relaxed);
        result
    }
}
