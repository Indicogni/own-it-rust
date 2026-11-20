//! WITNESS 3 — `deliver` asks more of its transport (change T4)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!   - pub fn deliver<T: Transport, M: Display>(
//!   + pub fn deliver<T: Transport + Send, M: Display>(
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
