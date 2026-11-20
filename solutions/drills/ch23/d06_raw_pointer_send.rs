//! DRILL 23-06 · Raw pointers across threads
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0277
//!
//! Run `cargo drill ch23`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the compiler know about who else uses this pointer?

use std::thread;

// FIX: a raw pointer is not `Send`, because the compiler does not know who
// owns the pointee or how long it lives. Move the owner into the thread: a
// `Box<i32>` is `Send`, and no unsafe code is needed.
fn main() {
    let data = Box::new(7);
    let seen = thread::spawn(move || *data).join().unwrap();
    assert_eq!(seen, 7); // KEEP
}
