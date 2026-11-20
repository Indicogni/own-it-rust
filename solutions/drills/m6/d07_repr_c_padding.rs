//! DRILL M6-07 · Translating a struct
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: panic
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `repr(C)` keeps the order of the fields. Which order did the header have?

// The C header says:
//
//     struct record {
//         uint64_t id;
//         uint8_t  tag;
//         uint8_t  flag;
//     };   /* sizeof(struct record) == 16 */
// FIX: with `repr(C)` the fields lie in the order in which they are written,
// each at the next offset that fits its alignment. `tag, id, flag` needs
// 1 + 7 (padding) + 8 + 1 + 7 (padding) = 24 bytes, and worse, every field
// but the first is at a different offset than in the C struct. The
// translation must list the fields in the header's order. (Without `repr(C)`
// Rust would be free to reorder them, and the size might happen to be 16
// while the offsets are still not C's.)
#[repr(C)]
struct Record {
    id: u64,
    tag: u8,
    flag: u8,
}

fn main() {
    assert_eq!(size_of::<Record>(), 16); // KEEP
    let record = Record {
        tag: 1,
        id: 2,
        flag: 3,
    };
    assert_eq!((record.id, record.tag, record.flag), (2, 1, 3)); // KEEP
}
