//! DRILL M5-09 · A bound on the impl, not on the type
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m5`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: the derive adds `T: Debug` somewhere. Is it on `struct Wrapper<T>` or on the impl of `Debug`?

/// No `Debug`, on purpose.
struct Opaque;

#[derive(Debug)]
struct Wrapper<T> {
    label: &'static str,
    inner: T,
}

fn main() {
    let plain = Wrapper {
        label: "plain",
        inner: 7,
    }; // KEEP
    let opaque = Wrapper {
        label: "opaque",
        inner: Opaque,
    }; // KEEP
    assert_eq!(
        format!("{plain:?}"),
        r#"Wrapper { label: "plain", inner: 7 }"#
    ); // KEEP
    assert_eq!(opaque.label, "opaque"); // KEEP
    let _ = opaque.inner; // KEEP
}
