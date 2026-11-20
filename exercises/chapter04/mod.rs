// `unused_imports` is allowed because `guess` is no longer used by a lesson once
// you have answered all of its predictions.
#![allow(dead_code, unused_variables, unused_mut, unused_imports)]

mod c01_code_review;
mod l01_scope;
mod l02_move_paths;
mod l03_control_flow;
mod l04_indexed;
mod l05_replace;
mod l06_copy_clone;
mod l07_shared;
mod l08_capstone;
mod r01_review;

// Lets tests count heap allocations, so they can check how you solved a task.
#[global_allocator]
static ALLOC: practice::alloc::Counting = practice::alloc::Counting;

/// A test helper for the drop-order predictions. You do not need to edit it,
/// and you do not need to understand how it is built yet: it uses ideas from
/// later chapters. You only need to know what its three methods do:
///
/// - `log.noisy("name")` makes a value. At the moment that value is dropped,
///   it writes `"name"` into `log`.
/// - `log.note("text")` writes `"text"` into `log` immediately.
/// - `log.entries()` returns everything written so far, oldest first.
mod droplog {
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A list of names, in the order in which they were written.
    #[derive(Default)]
    pub struct DropLog(Rc<RefCell<Vec<&'static str>>>);

    /// A value that writes its name into a [`DropLog`] when it is dropped.
    pub struct Noisy {
        name: &'static str,
        log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl DropLog {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn noisy(&self, name: &'static str) -> Noisy {
            Noisy {
                name,
                log: Rc::clone(&self.0),
            }
        }

        pub fn note(&self, text: &'static str) {
            self.0.borrow_mut().push(text);
        }

        pub fn entries(&self) -> Vec<&'static str> {
            self.0.borrow().clone()
        }
    }

    impl Drop for Noisy {
        fn drop(&mut self) {
            self.log.borrow_mut().push(self.name);
        }
    }
}
