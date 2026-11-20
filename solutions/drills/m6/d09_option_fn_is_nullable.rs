//! DRILL M6-09 · A callback that may be absent
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ok
//!
//! Run `cargo drill m6`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: how many bytes does C use for a function pointer that may be NULL?

/// `void (*callback)(void)`, which C allows to be NULL.
type Callback = Option<extern "C" fn()>;

extern "C" fn noop() {}

// Nothing to fix: this compiles and runs. A Rust function pointer is never
// null, so `Option` can use the all-zero pattern for `None` and needs no extra
// byte. The language guarantees this for `Option` of a function pointer (and
// of `&T`, `&mut T`, `Box<T>` and `NonNull<T>`): the type has the size and the
// ABI of a plain C pointer, and `None` is NULL. That is why a nullable C
// callback is declared as `Option<extern "C" fn(...)>`.
fn main() {
    let absent: Callback = None;
    let present: Callback = Some(noop);
    // SAFETY: both types are 8 bytes on a 64-bit target, and every bit
    // pattern is a valid usize.
    let (absent_bits, present_bits): (usize, usize) =
        unsafe { (std::mem::transmute(absent), std::mem::transmute(present)) };
    assert_eq!(size_of::<Callback>(), size_of::<usize>()); // KEEP
    assert_eq!(absent_bits, 0); // KEEP
    assert_ne!(present_bits, 0); // KEEP
}
