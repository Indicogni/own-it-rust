//! DRILL M6-07 · Translating a struct
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
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
#[repr(C)]
struct Record {
    tag: u8,
    id: u64,
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
