//! Raw bindings to the vendored `fletch` C library (lesson 02 lab).
//!
//! A `-sys` crate declares what the C header declares, and nothing more:
//! no safe wrappers, no conversions, no opinions.

#![no_std]

// TODO 3: declare the functions of vendor/fletch/include/fletch.h in an
// `unsafe extern "C"` block, as `pub` items. `fletch32` exists in the C
// library only in a build with FLETCH_WIDE, so its declaration must exist
// only when the feature `wide` is enabled.
