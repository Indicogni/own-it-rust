//! Run: cargo +nightly-2026-11-20 fuzz run parse_line
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(line) = std::str::from_utf8(data) {
        // Whatever the text is, the parser must return, and an accepted line must
        // survive being written out and parsed again.
        if let Ok(entry) = tally_core::parse_line(line) {
            let written = format!(
                "{} {} {}.{:02}",
                entry.date,
                entry.category,
                entry.cents / 100,
                entry.cents % 100
            );
            assert_eq!(tally_core::parse_line(&written), Ok(entry));
        }
    }
});
