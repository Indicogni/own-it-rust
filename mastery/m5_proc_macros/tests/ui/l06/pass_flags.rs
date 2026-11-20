//! Lesson 06: `flags!` in use.
//! This program must compile and run.

use m5_proc_macros::flags;

flags! {
    /// File rights.
    pub struct Perm: u8 {
        /// May read.
        READ,
        WRITE,
        EXEC,
    }
}

flags! {
    struct Empty: u64 {}
}

flags! {
    pub(crate) struct Full: u8 { A, B, C, D, E, F, G, H }
}

flags! {
    struct Wide: u64 {
        B00, B01, B02, B03, B04, B05, B06, B07, B08, B09, B10, B11, B12, B13, B14, B15,
        B16, B17, B18, B19, B20, B21, B22, B23, B24, B25, B26, B27, B28, B29, B30, B31,
        B32, B33, B34, B35, B36, B37, B38, B39, B40, B41, B42, B43, B44, B45, B46, B47,
        B48, B49, B50, B51, B52, B53, B54, B55, B56, B57, B58, B59, B60, B61, B62, B63
    }
}

// Everything except `names` works at compile time.
const READ_WRITE: Perm = Perm::READ.with(Perm::WRITE);
const ALL_BITS: u8 = Perm::all().bits();
const _: () = assert!(Perm::empty().bits() == 0);
const _: () = assert!(READ_WRITE.contains(Perm::WRITE));
const _: () = assert!(!READ_WRITE.contains(Perm::EXEC));
const _: () = assert!(Full::all().bits() == u8::MAX);
const _: () = assert!(Wide::B63.bits() == 1 << 63);

fn main() {
    assert_eq!(Perm::READ.bits(), 1);
    assert_eq!(Perm::WRITE.bits(), 2);
    assert_eq!(Perm::EXEC.bits(), 4);
    assert_eq!(ALL_BITS, 7);

    // Copy, PartialEq and Debug.
    let copy = READ_WRITE;
    assert_eq!(copy, READ_WRITE);
    assert_ne!(copy, Perm::all());
    assert_eq!(format!("{:?}", Perm::EXEC), "Perm(4)");

    assert!(Perm::all().contains(READ_WRITE));
    assert!(
        READ_WRITE.contains(Perm::empty()),
        "every set contains the empty set"
    );
    assert!(!Perm::READ.contains(READ_WRITE));
    assert_eq!(Perm::READ.with(Perm::READ), Perm::READ);

    assert_eq!(READ_WRITE.names(), ["READ", "WRITE"]);
    assert_eq!(
        Perm::EXEC.with(Perm::READ).names(),
        ["READ", "EXEC"],
        "declaration order"
    );
    assert!(Perm::empty().names().is_empty());

    assert_eq!(Empty::all(), Empty::empty());
    assert_eq!(Empty::all().bits(), 0);
    assert!(Empty::all().names().is_empty());
    assert_eq!(Full::H.bits(), 128);
    assert_eq!(Wide::all().bits(), u64::MAX);
    assert_eq!(Wide::B63.names(), ["B63"]);
}
