//! DRILL 20-06 · A future nobody polls
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ok
//!
//! Run `cargo drill ch20`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: when does an async block's body run?

// Nothing to fix: this program compiles and runs. A future is lazy. The
// block's body, with its `sent += 1`, runs only when something polls the
// future. Dropping an unpolled future only releases what it captured.
fn main() {
    let mut sent = 0;
    let send_email = async {
        sent += 1;
    };
    drop(send_email);
    assert_eq!(sent, 0); // KEEP
}
