//! DRILL 05-12 · Passing `&mut`: lent again, or given away?
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0382
//!
//! Run `cargo drill ch05`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `&mut i32` is not Copy. Why may `r` be passed to `bump` twice, and
//! what is different about the parameter of `describe`?

fn bump(n: &mut i32) {
    *n += 1;
}

fn describe<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}")
}

fn main() {
    let mut count = 0;
    let r = &mut count;
    bump(r);
    bump(r);
    // FIX: `describe` takes a generic `T`, so passing `r` itself moves the
    // `&mut i32` into the call. Lend it instead. A shared reborrow `&*r` is
    // enough because `describe` only reads; `&mut *r` also works.
    let seen = describe(&*r);
    bump(r); // KEEP
    assert_eq!(seen, "2"); // KEEP
    assert_eq!(count, 3); // KEEP
}
