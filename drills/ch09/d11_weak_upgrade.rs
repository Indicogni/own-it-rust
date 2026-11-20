//! DRILL 09-11 · Upgrading a Weak pointer
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: ?
//!
//! Run `cargo drill ch09`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: who still owns the team name when the last two lines run?

use std::rc::{Rc, Weak};

struct Member {
    team: Weak<String>,
}

fn main() {
    let member;
    {
        let team = Rc::new(String::from("compilers"));
        member = Member { team: Rc::downgrade(&team) };
        assert_eq!(Rc::weak_count(&team), 1); // KEEP
    }
    let team = member.team.upgrade().unwrap();
    assert_eq!(team.as_str(), "compilers"); // KEEP
}
