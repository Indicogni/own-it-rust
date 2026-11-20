//! DRILL M4-02 · Pinning a value that must not move
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0277
//!
//! Run `cargo drill m4`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: which promise would `Pin::new` break for a type that is not `Unpin`?

use std::marker::PhantomPinned;
use std::pin::Pin;

/// A value that other code points into, so it must stay where it is.
struct Anchor {
    label: String,
    _pinned: PhantomPinned,
}

impl Anchor {
    fn new(label: &str) -> Anchor {
        Anchor {
            label: label.to_owned(),
            _pinned: PhantomPinned,
        }
    }

    fn label(self: Pin<&Self>) -> &str {
        &self.get_ref().label
    }
}

fn main() {
    let anchor = Anchor::new("stay"); // KEEP
    // FIX: `Pin::new` is only for `Unpin` types, because its caller could
    // move the value again afterwards. `pin!` takes the value over, so
    // nothing can move it any more.
    let pinned = std::pin::pin!(anchor);
    assert_eq!(pinned.as_ref().label(), "stay"); // KEEP
}
