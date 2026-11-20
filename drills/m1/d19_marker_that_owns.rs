//! DRILL M1-19 · A marker that says "I own one"
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: ?
//!
//! Run `cargo drill m1`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: `Ticket` has no `Drop` impl. What does the compiler think may run when a `Ticket` is dropped?

use std::marker::PhantomData;

struct Guard<'a>(&'a str);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        println!("guard for {} released", self.0);
    }
}

struct Ticket<'a> {
    seat: String,
    _event: PhantomData<Guard<'a>>,
}

fn ticket_for<'a>(_event: &'a str, seat: &str) -> Ticket<'a> {
    Ticket { seat: seat.to_string(), _event: PhantomData }
}

fn main() {
    let ticket;
    let event = String::from("concert");
    ticket = ticket_for(&event, "12A"); // KEEP
    assert_eq!(ticket.seat, "12A"); // KEEP
}
