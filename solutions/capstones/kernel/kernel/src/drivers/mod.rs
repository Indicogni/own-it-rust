//! The driver layer.
//!
//! The rest of the kernel talks to hardware only through the traits in this
//! file. Each driver is a struct that owns the register block of one device
//! and implements one trait; `board.rs` decides which driver sits at which
//! address. Porting the kernel to another board means writing new drivers
//! and a new `board.rs`, not touching the scheduler or the allocator.

pub mod generic_timer;
pub mod gicv2;
pub mod pl011;

/// A character output device.
pub trait Console: Sync {
    /// A short name for log lines.
    fn name(&self) -> &'static str;

    /// Sends one byte, waiting until the device can take it.
    fn write_byte(&self, byte: u8);

    /// Waits until every byte sent so far has left the device.
    fn flush(&self);

    /// Sends a string. Serial terminals expect a carriage return before
    /// each line feed, so `\n` goes out as `\r\n`.
    fn write_str(&self, text: &str) {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

/// A free-running counter that can also raise a periodic interrupt.
pub trait Timer: Sync {
    fn name(&self) -> &'static str;

    /// How many times per second the counter increments.
    fn frequency_hz(&self) -> u64;

    /// The current counter value. It only ever grows.
    fn now(&self) -> u64;

    /// Starts raising the timer interrupt `tick_hz` times per second.
    fn start_periodic(&self, tick_hz: u64);

    /// Called by the interrupt handler: silences the current interrupt and
    /// schedules the next one.
    fn acknowledge(&self);

    /// Stops the periodic interrupt.
    fn stop(&self);
}

/// The device that collects interrupt lines and presents them to the CPU.
pub trait InterruptController: Sync {
    /// Brings the controller into a state where enabled interrupts reach
    /// the CPU.
    fn init(&self);

    /// Lets interrupt `intid` through.
    fn enable(&self, intid: u32);

    /// Asks which interrupt is pending and marks it as being handled.
    /// `None` means there was nothing to handle (a spurious interrupt).
    fn claim(&self) -> Option<u32>;

    /// Tells the controller that the handler for `intid` has finished.
    fn complete(&self, intid: u32);
}
