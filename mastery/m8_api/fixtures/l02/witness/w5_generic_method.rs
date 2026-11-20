//! WITNESS 5 — `Transport` gains a provided, generic method (change T7)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!     pub trait Transport {
//!         fn send(&mut self, bytes: &[u8]) -> usize;
//!   +     fn send_all<I: IntoIterator<Item = Vec<u8>>>(&mut self, chunks: I) -> usize {
//!   +         chunks.into_iter().map(|chunk| self.send(&chunk)).sum()
//!   +     }
//!     }
//!
//! The new method has a body, and its name is unlikely to clash with anything.
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
