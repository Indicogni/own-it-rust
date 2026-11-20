//! The `game` binary. SPEC section 14 says what it must do:
//!
//! - with the `window` feature (on by default): open a window and play;
//! - with `--headless --replay FILE`: run a replay without a window and
//!   print one line per step. This mode must work without the feature.
//!
//! Keep the window crate out of the library: only code in this binary,
//! behind `#[cfg(feature = "window")]`, may name it.

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("game: not implemented yet. Start with SPEC.md, milestone M1.");
    ExitCode::from(2)
}
