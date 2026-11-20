//! Structure-aware target: libFuzzer's bytes become a `Query` through your
//! `Arbitrary` impl, and your `fuzz_roundtrip` checks the shipped codec with it.
//! Run: cargo +nightly-2026-11-20 fuzz run roundtrip
#![no_main]

use libfuzzer_sys::fuzz_target;
use m9_toolbox::l05_fuzz::{BUGGY, Query, fuzz_roundtrip};

fuzz_target!(|query: Query| {
    fuzz_roundtrip(&query, BUGGY);
});
