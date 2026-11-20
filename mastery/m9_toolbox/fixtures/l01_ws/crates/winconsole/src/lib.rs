//! A stand-in for a Windows-only helper crate. A real one would switch the console
//! to ANSI escape mode through the Windows API.

/// Does nothing here. Only `meter-cli` built for Windows calls it.
pub fn enable_ansi() {}
