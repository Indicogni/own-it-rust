//! WITNESS 1 — `Transport` gains a required method (change T1)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!     pub trait Transport {
//!         fn send(&mut self, bytes: &[u8]) -> usize;
//!   +     fn flush(&mut self);
//!     }
//!
//! Write a program that is correct for courier 1.0.0 (it compiles, runs, and
//! its assertions hold) and that the compiler rejects when the library has
//! this change. Such a program is a witness: it proves that the change can
//! break a user. Then replace the `?` by the error code that you expect:
//!
//! EXPECT: E0046

use courier::{Plain, Transport, deliver};

/// A transport of this program: it only counts.
struct Counting(usize);

impl Transport for Counting {
    fn send(&mut self, bytes: &[u8]) -> usize {
        self.0 += bytes.len();
        bytes.len()
    }
}

fn main() {
    let mut transport = Counting(0);
    assert_eq!(deliver(&mut transport, &Plain, "abc"), 3);
    assert_eq!(transport.0, 3);
}
