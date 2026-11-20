//! WITNESS 2 — `Transport` gains a provided method (change T2)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!     pub trait Transport {
//!         fn send(&mut self, bytes: &[u8]) -> usize;
//!   +     fn flush(&mut self) {}
//!     }
//!
//! The new method has a body, so no implementation of the trait has to change.
//! Look for a program that breaks for another reason.
//!
//! Write a program that is correct for courier 1.0.0 (it compiles, runs, and
//! its assertions hold) and that the compiler rejects when the library has
//! this change. Such a program is a witness: it proves that the change can
//! break a user. Then replace the `?` by the error code that you expect:
//!
//! EXPECT: ?

fn main() {
    todo!("write the witness")
}
