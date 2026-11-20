//! DRILL M1-07 · Two items from a lending iterator
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0499
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does each item borrow, and for how long?

trait Lender {
    type Item<'a>
    where
        Self: 'a;
    fn next(&mut self) -> Option<Self::Item<'_>>;
}

struct Lines {
    buffer: String,
    left: u32,
}

impl Lender for Lines {
    type Item<'a> = &'a str;

    fn next(&mut self) -> Option<&str> {
        if self.left == 0 {
            return None;
        }
        self.left -= 1;
        self.buffer = format!("line {}", self.left);
        Some(&self.buffer)
    }
}

// FIX: each item borrows the buffer inside `lines`, and it keeps the
// `&mut lines` of its `next` call alive. A second `next` would overwrite the
// buffer under the first item, so the compiler refuses it. To hold two lines
// at once, each one needs its own storage: copy the item before the next call.
fn main() {
    let mut lines = Lines { buffer: String::new(), left: 2 };
    let first = lines.next().unwrap().to_owned();
    let second = lines.next().unwrap().to_owned();
    assert_eq!(first, "line 1"); // KEEP
    assert_eq!(second, "line 0"); // KEEP
}
