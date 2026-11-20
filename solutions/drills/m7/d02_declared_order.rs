//! DRILL M7-02 · Fields in declared order
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m7`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: where does each field start when the compiler may not reorder them?

// FIX: with `repr(C)` the fields lie in declared order. `offset` needs an
// address that is a multiple of 8, so 7 bytes of padding followed `tag`, and
// `kind` and `len` filled 8 more bytes: 24 in total. The program compiled, and
// the size assertion panicked. Ordered from the largest alignment to the
// smallest, the fields need no padding.
// The layout is shared with a C program.
#[repr(C)] // KEEP
struct Entry {
    // Largest alignment first, so that no field needs padding in front of it.
    offset: u64,
    len: u32,
    tag: u8,
    kind: u8,
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
