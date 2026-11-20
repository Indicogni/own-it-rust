//! DRILL M2-04 · Taking a field out of a type with a destructor
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0509
//!
//! Run `cargo drill m2`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what would the destructor find in `self.user` after the move?

struct Session {
    user: String,
}

impl Drop for Session {
    fn drop(&mut self) {
        println!("closing the session of {:?}", self.user);
    }
}

// FIX: a type with a `Drop` impl must be whole when its destructor runs, so a
// field cannot be moved out of it. `mem::take` swaps an empty String in, and
// the destructor then sees that empty name. (A container such as `Box` cannot
// leave a "default" behind. It reads the value out with `ptr::read` and then
// stops its own destructor with `ManuallyDrop` or `mem::forget`: lesson 02.)
fn into_user(session: Session) -> String {
    let mut session = session;
    std::mem::take(&mut session.user)
}

fn main() {
    let session = Session {
        user: String::from("ada"),
    };
    assert_eq!(into_user(session), "ada"); // KEEP
}
