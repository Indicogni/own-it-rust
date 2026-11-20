//! DRILL M6-05 · A borrowed view and a later write
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0502
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the library says: the pointer is valid until the next put. Who checks that here?

/// A table whose `peek` returns a view into its storage, as a C library's
/// "pointer valid until the next put" would.
struct Table {
    values: Vec<Vec<u8>>,
}

impl Table {
    fn peek(&self, index: usize) -> &[u8] {
        &self.values[index]
    }

    fn put(&mut self, index: usize, value: &[u8]) {
        self.values[index] = value.to_vec();
    }
}

// FIX: the view borrows the table, and `put` needs the table exclusively, so
// the compiler refuses to free the bytes that `view` still points to. That is
// the wrapper's design at work: `peek(&self) -> &[u8]` and `put(&mut self)`
// turn the library's sentence "valid until the next put" into a rule that the
// compiler checks. To keep the old bytes past the write, copy them.
fn main() {
    let mut table = Table {
        values: vec![b"old".to_vec()],
    };
    let view = table.peek(0).to_vec();
    table.put(0, b"new");
    assert_eq!(view, b"old"); // KEEP
    assert_eq!(table.peek(0), b"new"); // KEEP
}
