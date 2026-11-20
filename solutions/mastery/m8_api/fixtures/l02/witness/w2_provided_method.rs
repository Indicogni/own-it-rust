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
//! EXPECT: E0034

use courier::{Memory, Transport};

/// This program had its own idea of "flush" for the library's `Memory` type.
trait Drain {
    fn flush(&mut self) -> usize;
}

impl Drain for Memory {
    fn flush(&mut self) -> usize {
        let bytes = self.sent.len();
        self.sent.clear();
        bytes
    }
}

fn main() {
    let mut memory = Memory::default();
    memory.send(b"abc");
    // With the change, two traits in scope offer a method `flush` for `Memory`.
    assert_eq!(memory.flush(), 3);
}
