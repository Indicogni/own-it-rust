//! Driver for the Arm generic timer, using the EL1 virtual timer.
//!
//! This device has no memory-mapped registers at all: it is part of the
//! CPU and is driven through system registers (`mrs`/`msr`). The registers
//! are described in the "Arm Architecture Reference Manual for A-profile"
//! (Arm DDI 0487), chapter D11 "The Generic Timer".

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use kcore::reg::timer_interval;

use super::Timer;

/// `CNTV_CTL_EL0` bit 0: the timer is enabled. Bit 1 (IMASK) stays 0 so
/// that the interrupt is delivered.
const CTL_ENABLE: u64 = 1;

pub struct GenericTimer {
    /// Counter ticks between two interrupts; set by `start_periodic`.
    interval: AtomicU64,
}

impl GenericTimer {
    pub const fn new() -> Self {
        Self {
            interval: AtomicU64::new(0),
        }
    }

    /// Fires the timer `ticks` counter ticks from now.
    fn arm(&self, ticks: u64) {
        // SAFETY: writing the virtual timer's value register is allowed at
        // EL1. It changes only the time of the next timer interrupt.
        unsafe {
            asm!("msr cntv_tval_el0, {}", in(reg) ticks, options(nomem, nostack, preserves_flags));
        }
    }

    fn set_control(&self, value: u64) {
        // SAFETY: writing the virtual timer's control register is allowed
        // at EL1. It only enables or disables the timer interrupt source.
        unsafe {
            asm!("msr cntv_ctl_el0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
        }
    }
}

impl Timer for GenericTimer {
    fn name(&self) -> &'static str {
        "arm generic timer"
    }

    fn frequency_hz(&self) -> u64 {
        let hz: u64;
        // SAFETY: `CNTFRQ_EL0` is readable at every exception level and the
        // read has no side effects.
        unsafe {
            asm!("mrs {}, cntfrq_el0", out(reg) hz, options(nomem, nostack, preserves_flags));
        }
        hz
    }

    fn now(&self) -> u64 {
        let count: u64;
        // SAFETY: reading the virtual counter has no side effects. The
        // `isb` makes sure the read is not executed ahead of earlier
        // instructions, so two calls in a row never go backwards.
        unsafe {
            asm!("isb", "mrs {}, cntvct_el0", out(reg) count, options(nomem, nostack, preserves_flags));
        }
        count
    }

    fn start_periodic(&self, tick_hz: u64) {
        let interval = timer_interval(self.frequency_hz(), tick_hz);
        self.interval.store(interval, Ordering::Relaxed);
        self.arm(interval);
        self.set_control(CTL_ENABLE);
    }

    fn acknowledge(&self) {
        // The interrupt line stays high while "counter >= compare value"
        // holds. Moving the compare value into the future lowers the line
        // and schedules the next tick. Counting from *now* (not from the
        // previous deadline) guarantees a full interval of progress between
        // two ticks, even if the emulator was paused for a while.
        self.arm(self.interval.load(Ordering::Relaxed));
    }

    fn stop(&self) {
        self.set_control(0);
    }
}
