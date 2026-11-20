//! DRILL M7-02 · Fields in declared order
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does each field start when the compiler may not reorder them?

// The layout is shared with a C program.
#[repr(C)] // KEEP
struct Entry {
    tag: u8,
    offset: u64,
    kind: u8,
    len: u32,
}

fn main() {
    let entry = Entry {
        tag: 1,
        offset: 2,
        kind: 3,
        len: 4,
    };
    let sum = entry.offset + u64::from(entry.len) + u64::from(entry.tag + entry.kind);
    assert_eq!(sum, 10); // KEEP
    assert_eq!(std::mem::size_of::<Entry>(), 16); // KEEP
}
