//! Bytes-in target: feeds raw fuzzer input to the shipped decoder.
//! Run: cargo +nightly-2026-11-20 fuzz run decode
#![no_main]

use libfuzzer_sys::fuzz_target;
use m9_toolbox::l05_fuzz::{BUGGY, fuzz_decode};

fuzz_target!(|data: &[u8]| {
    fuzz_decode(data, BUGGY);
});
