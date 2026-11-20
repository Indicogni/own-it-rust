//! DRILL 17-05 · push versus push_str
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: E0308
//!
//! Run `cargo drill ch17`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what is the difference between 'a' and "a"?

fn main() {
    let mut path = String::from("/tmp");
    // FIX: `push` takes one `char` (single quotes); `push_str` takes a `&str`.
    path.push('/');
    path.push_str("out");
    assert_eq!(path, "/tmp/out"); // KEEP
}
