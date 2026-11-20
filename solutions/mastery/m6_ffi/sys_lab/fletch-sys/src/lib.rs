//! Raw bindings to the vendored `fletch` C library (lesson 02 lab).
//!
//! A `-sys` crate declares what the C header declares, and nothing more:
//! no safe wrappers, no conversions, no opinions.

#![no_std]

use core::ffi::c_int;

// SAFETY: translated from vendor/fletch/include/fletch.h. `uint8_t`,
// `uint16_t` and `uint32_t` are exactly `u8`, `u16` and `u32`; `size_t` is
// `usize`; `int` is `c_int`.
unsafe extern "C" {
    /// `uint16_t fletch16(const uint8_t *data, size_t len);`
    pub fn fletch16(data: *const u8, len: usize) -> u16;

    /// `uint32_t fletch32(const uint8_t *data, size_t len);`
    ///
    /// The symbol exists only when the C code was compiled with
    /// `FLETCH_WIDE`, so the declaration sits behind the same feature that
    /// makes build.rs define the macro.
    #[cfg(feature = "wide")]
    pub fn fletch32(data: *const u8, len: usize) -> u32;

    /// `int fletch_has_wide(void);` No arguments and no preconditions, so
    /// calling it is safe.
    pub safe fn fletch_has_wide() -> c_int;
}
