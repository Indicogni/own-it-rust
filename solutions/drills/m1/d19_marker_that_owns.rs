//! DRILL M1-19 · A marker that says "I own one"
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0308.
//! EXPECT: E0597
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

// FIX: `Ticket` has no `Drop` impl, but its `String` must be freed, so the
// drop checker goes through the fields. It treats `PhantomData<Guard<'a>>`
// like a `Guard<'a>` field: "dropping a ticket may run `Drop for Guard`, which
// reads through 'a". `event` is dropped before `ticket`, so that is E0597.
// A ticket owns no guard; it only remembers the borrow. The marker for that
// is `PhantomData<&'a str>`, and dropping a reference runs no code.
// (Declaring `event` before `ticket` also works. It hides the wrong marker.)
struct Ticket<'a> {
    seat: String,
    _event: PhantomData<&'a str>,
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
