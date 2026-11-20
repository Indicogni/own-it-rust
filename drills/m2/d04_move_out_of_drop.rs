//! DRILL M2-04 · Taking a field out of a type with a destructor
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
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

fn into_user(session: Session) -> String {
    session.user
}

fn main() {
    let session = Session {
        user: String::from("ada"),
    };
    assert_eq!(into_user(session), "ada"); // KEEP
}
