//! `print!` and `println!` for the kernel.

use core::fmt::{self, Write};

use crate::board;
use crate::cpu::IrqGuard;

/// Adapts the board's console to `core::fmt`.
struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        board::console().write_str(text);
        Ok(())
    }
}

/// Implementation detail of the macros.
///
/// The whole message goes out with IRQs masked. That keeps an interrupt
/// handler, or a task that the timer switches to, from printing in the
/// middle of the line.
pub fn _print(args: fmt::Arguments) {
    let _irq = IrqGuard::new();
    // `ConsoleWriter` never fails; only a `Display` impl could return an error.
    let _ = ConsoleWriter.write_fmt(args);
}

macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::_print(format_args!($($arg)*))
    };
}

macro_rules! println {
    () => {
        $crate::console::_print(format_args!("\n"))
    };
    ($($arg:tt)*) => {
        // One call for text and line end, so the line cannot be torn apart.
        $crate::console::_print(format_args!("{}\n", format_args!($($arg)*)))
    };
}
