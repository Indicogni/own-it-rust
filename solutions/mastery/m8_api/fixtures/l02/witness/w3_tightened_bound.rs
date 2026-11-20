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
//! EXPECT: E0277

use courier::{Plain, Transport, deliver};
use std::cell::RefCell;
use std::rc::Rc;

/// A transport that shares its buffer with the rest of this program. `Rc` is
/// not `Send`, so neither is `Shared`.
struct Shared(Rc<RefCell<Vec<u8>>>);

impl Transport for Shared {
    fn send(&mut self, bytes: &[u8]) -> usize {
        self.0.borrow_mut().extend_from_slice(bytes);
        bytes.len()
    }
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut transport = Shared(Rc::clone(&log));
    assert_eq!(deliver(&mut transport, &Plain, 42), 2);
    assert_eq!(*log.borrow(), b"42");
}
