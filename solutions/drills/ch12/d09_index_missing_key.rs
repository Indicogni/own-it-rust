//! DRILL 12-09 · Indexing a map with a missing key
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch12`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what can Index return when there is nothing to point at?

use std::collections::HashMap;

fn main() {
    let ports = HashMap::from([("http", 80), ("https", 443)]);
    // FIX: `ports["ssh"]` panics, because `Index` has no way to say "absent".
    // `get` returns an Option, and the caller decides what a missing key means:
    // here, the standard port.
    let ssh = ports.get("ssh").copied().unwrap_or(22);
    assert_eq!(ssh, 22); // KEEP
}
