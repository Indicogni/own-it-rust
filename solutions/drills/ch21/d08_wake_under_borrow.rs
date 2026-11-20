//! DRILL 21-08 · Waking while the state is borrowed
//!
//! PREDICT without compiling: what happens when this program is built and run?
//! Answer ok, panic, error, or the error code(s), e.g. E0382.
//! EXPECT: panic
//!
//! Run `cargo drill ch21`. Then FIX it with the smallest change that keeps
//! every line marked KEEP and makes the program run cleanly.
//! Ask: what does the woken task do first?

use std::cell::RefCell;
use std::rc::Rc;

/// A single-threaded stand-in for a waker: "waking" runs the task's poll,
/// which inspects the shared state.
struct Slot {
    value: Option<u32>,
    on_wake: Option<Box<dyn FnOnce()>>,
}

fn complete(slot: &Rc<RefCell<Slot>>, value: u32) {
    // FIX: take the callback out while the state is borrowed, end the borrow,
    // and only then call it. The woken task can now borrow the state itself.
    let wake = {
        let mut state = slot.borrow_mut();
        state.value = Some(value);
        state.on_wake.take()
    };
    if let Some(wake) = wake {
        wake();
    }
}

fn main() {
    let slot = Rc::new(RefCell::new(Slot {
        value: None,
        on_wake: None,
    }));
    let seen = Rc::new(RefCell::new(None));
    let (task_slot, task_seen) = (Rc::clone(&slot), Rc::clone(&seen));
    slot.borrow_mut().on_wake = Some(Box::new(move || {
        *task_seen.borrow_mut() = task_slot.borrow().value;
    }));
    complete(&slot, 9);
    assert_eq!(*seen.borrow(), Some(9)); // KEEP
}
