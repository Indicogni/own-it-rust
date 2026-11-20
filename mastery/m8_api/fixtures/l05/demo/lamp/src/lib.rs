//! A library with one feature. You do not need to edit this file.

/// How much light the lamp gives.
pub fn lumens() -> u32 {
    if cfg!(feature = "bright") { 900 } else { 400 }
}
