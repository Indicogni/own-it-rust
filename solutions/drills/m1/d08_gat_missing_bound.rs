//! DRILL M1-08 · A generic associated type without its bound
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: error
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: an item of type `Item<'a>` borrows from `Self`. What must be true about `Self` and 'a?

// FIX: `next` returns `Self::Item<'_>` with the lifetime of `&mut self`, so
// the item may borrow from `Self` for 'a. That only makes sense when `Self`
// is valid for 'a. The compiler requires the bound to be written on the
// associated type ("missing required bound on `Item`"; this error has no
// code), so that every impl may rely on it.
trait Lender {
    type Item<'a>
    where
        Self: 'a;
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

struct Countdown {
    left: u32,
    text: String,
}

impl Lender for Countdown {
    type Item<'a> = &'a str;

    fn next(&mut self) -> Option<&str> {
        self.left = self.left.checked_sub(1)?;
        self.text = self.left.to_string();
        Some(&self.text)
    }
}

fn main() {
    let mut countdown = Countdown { left: 2, text: String::new() };
    assert_eq!(countdown.next(), Some("1")); // KEEP
    assert_eq!(countdown.next(), Some("0")); // KEEP
    assert_eq!(countdown.next(), None); // KEEP
}
