//! WITNESS 4 — a private field of `Stats` changes its type (change T6)
//!
//! The change to `fixtures/l02/courier.rs`:
//!
//!     pub struct Stats {
//!   -     messages: Arc<AtomicUsize>,
//!   +     messages: Rc<Cell<usize>>,
//!     }
//!
//! No public signature changes: `new`, `record`, `messages` and `Clone` are as
//! before, and they behave as before in a program with one thread.
//!
//! Write a program that is correct for courier 1.0.0 (it compiles, runs, and
//! its assertions hold) and that the compiler rejects when the library has
//! this change. Such a program is a witness: it proves that the change can
//! break a user. Then replace the `?` by the error code that you expect:
//!
//! EXPECT: E0277

use courier::Stats;

fn main() {
    let stats = Stats::new();
    let worker = stats.clone();
    // Moving a `Stats` to another thread needs `Stats: Send`.
    std::thread::spawn(move || worker.record())
        .join()
        .expect("the worker does not panic");
    assert_eq!(stats.messages(), 1);
}
