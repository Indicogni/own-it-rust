//! LESSON 05 — Callbacks: userdata, trampolines, closures that C owns, and panics at the boundary
//! Read first: CONCEPTS.md, "Lesson 05". Start after: lesson 04.
//! Run: cargo test --manifest-path mastery/m6_ffi/Cargo.toml l05_
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace each
//! `guess()` with your answer). Then consider: C calls your callback with a
//! `void *userdata` and knows nothing about Rust closures. Where does the closure
//! live while `kvtab_for_each` runs? Where does it live after `kvtab_set_observer`
//! has returned, and who destroys it?
//! IMPLEMENT the tasks below. Preserve the tests and add an edge case.
//! EXPLAIN afterward: For each of your two trampolines, say what `userdata` points
//! to, who owns that memory, how long it lives, and why no second reference to it
//! can exist while the trampoline runs. Then say what happens, step by step, when
//! the closure panics: in `for_each`, and in an observer.
//! MIRI: this lesson runs under Miri against the Rust model of the library.
//!
//! This lesson adds methods to the `Table` of lesson 04, so its tests need your
//! lesson 04 code. The public signatures are given. The trampolines (the
//! `extern "C"` functions that C calls) and the data behind `userdata` are yours
//! to design; the tests never name them.

use crate::l04_safe_wrapper::{Error, Table};
use crate::sys;
use std::any::Any;
use std::ffi::{CStr, c_char, c_int, c_uchar, c_void};
use std::ops::ControlFlow;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

/// Supplied. What an observer is told about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A key was stored or replaced.
    Put,
    /// A key was removed.
    Remove,
}

impl Table {
    /// A. Calls `f` once for every entry, in insertion order, with the key
    /// and the value. The two borrows are valid only during the call.
    ///
    /// - `ControlFlow::Break(())` stops the walk. Stopping is not an error:
    ///   the result is `Ok(())`.
    /// - The closure may read this table (it can capture a `&Table`). It
    ///   cannot change it, because every changing method takes `&mut self`.
    /// - No heap allocation: the closure stays where it is, and `userdata`
    ///   points to it (or to a local value that contains it).
    /// - If `f` panics, the panic must not pass through the C function.
    ///   Catch it in the function that C calls, make the library stop the
    ///   walk, and continue the same panic (`resume_unwind`) after
    ///   `kvtab_for_each` has returned.
    pub fn for_each<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnMut(&CStr, &[u8]) -> ControlFlow<()>,
    {
        todo!("A: pass the closure through userdata and write the trampoline")
    }

    /// B. Installs `f` as the table's observer: the library calls it after
    /// every successful `put` (with `Event::Put`) and `remove`
    /// (`Event::Remove`), with the key. A previous observer is replaced.
    ///
    /// - The table owns the closure from now on. It is dropped exactly once:
    ///   when it is replaced, when [`Table::clear_observer`] is called, or
    ///   when the table is dropped. Use the drop function that
    ///   `kvtab_set_observer` accepts.
    /// - One heap allocation at most: box the closure itself. Do not put a
    ///   `Box<dyn FnMut>` into a second box.
    /// - `F: 'static`, because the closure lives as long as the table, and
    ///   `F: Send`, because a `Table` is `Send` and takes its observer along:
    ///
    /// ```compile_fail,E0277
    /// use m6_ffi::l04_safe_wrapper::Table;
    /// use std::rc::Rc;
    ///
    /// let mut table = Table::new(1).unwrap();
    /// let counter = Rc::new(std::cell::Cell::new(0));
    /// table.observe(move |_, _| counter.set(counter.get() + 1));
    /// ```
    ///
    /// - A panic in an observer cannot be handed to anybody: the caller is C.
    ///   This wrapper's policy is that such a panic aborts the process, which
    ///   is what an `extern "C"` function does by itself. An event code that
    ///   this wrapper does not know is ignored.
    pub fn observe<F>(&mut self, f: F)
    where
        F: FnMut(Event, &CStr) + Send + 'static,
    {
        todo!("B: give the closure to the table")
    }

    /// B. Removes the observer, if there is one, and drops its closure.
    pub fn clear_observer(&mut self) {
        todo!("B: remove the observer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::alloc::count;
    use crate::sys;
    use crate::{guess, within};
    // Imported here too, so that the tests do not depend on which imports
    // your implementation above still needs.
    use std::ffi::{CStr, c_char, c_int, c_uchar, c_void};
    use std::ops::ControlFlow;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, Mutex};

    fn table_with(entries: &[(&CStr, &[u8])]) -> Table {
        let mut table = Table::new(entries.len().max(1)).unwrap();
        for (key, value) in entries {
            table.put(key, value).unwrap();
        }
        table
    }

    #[test]
    fn a_visits_every_entry_in_insertion_order() {
        let table = table_with(&[(c"b", b"2"), (c"a", b"11"), (c"c", b"")]);
        let mut seen: Vec<(String, Vec<u8>)> = Vec::new();
        let result = table.for_each(|key, value| {
            seen.push((key.to_str().unwrap().to_string(), value.to_vec()));
            ControlFlow::Continue(())
        });
        assert_eq!(result, Ok(()));
        assert_eq!(
            seen,
            [
                ("b".to_string(), b"2".to_vec()),
                ("a".to_string(), b"11".to_vec()),
                ("c".to_string(), Vec::new()),
            ]
        );
    }

    #[test]
    fn a_an_empty_table_calls_nothing() {
        let table = Table::new(2).unwrap();
        let mut calls = 0;
        assert_eq!(
            table.for_each(|_, _| {
                calls += 1;
                ControlFlow::Continue(())
            }),
            Ok(())
        );
        assert_eq!(calls, 0);
    }

    #[test]
    fn a_break_stops_the_walk_and_is_not_an_error() {
        let table = table_with(&[(c"a", b"1"), (c"b", b"2"), (c"c", b"3")]);
        let mut calls = 0;
        let result = table.for_each(|key, _| {
            calls += 1;
            if key == c"b" {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        assert_eq!(
            result,
            Ok(()),
            "the library reports KVTAB_ERR_STOPPED; for the caller who asked \
             to stop, that is success"
        );
        assert_eq!(calls, 2, "the closure must not be called after Break");
    }

    #[test]
    fn a_the_closure_may_read_the_same_table() {
        let table = table_with(&[(c"a", b"1"), (c"b", b"22")]);
        let mut total = 0;
        table
            .for_each(|_, value| {
                total += table.len() + value.len();
                ControlFlow::Continue(())
            })
            .unwrap();
        assert_eq!(total, 2 + 1 + 2 + 2);
    }

    #[test]
    fn a_technique_the_closure_is_not_boxed() {
        let table = table_with(&[(c"a", b"1"), (c"b", b"2")]);
        let mut bytes = 0;
        let (result, allocations) = count(|| {
            table.for_each(|key, value| {
                bytes += key.to_bytes().len() + value.len();
                ControlFlow::Continue(())
            })
        });
        assert_eq!(result, Ok(()));
        assert_eq!(bytes, 4);
        assert_eq!(
            allocations, 0,
            "for_each needs no heap allocation: `userdata` can point to a \
             value on for_each's own stack"
        );
    }

    #[test]
    fn a_technique_a_panic_is_carried_over_the_c_frames() {
        // A panic that tries to leave an `extern "C"` function aborts the
        // process, so this test runs in a child process (see harness.rs).
        if crate::harness::in_child(
            "l05_callbacks::tests::a_technique_a_panic_is_carried_over_the_c_frames",
        ) {
            return;
        }
        let mut table = table_with(&[(c"a", b"1"), (c"b", b"2"), (c"c", b"3")]);
        let mut calls = 0;
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            table.for_each(|key, _| {
                calls += 1;
                if key == c"b" {
                    panic!("visitor failed on b");
                }
                ControlFlow::Continue(())
            })
        }));
        let payload = outcome.expect_err("the panic must reach the caller of for_each");
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"visitor failed on b"),
            "continue the original panic with its own payload"
        );
        assert_eq!(calls, 2, "the walk must stop at the panic");
        // The library was left in a consistent state.
        assert_eq!(table.len(), 3);
        table.remove(c"a").unwrap();
        assert_eq!(table.len(), 2);
    }

    type Log = Arc<Mutex<Vec<(Event, String)>>>;

    fn logging_observer(log: &Log) -> impl FnMut(Event, &CStr) + Send + 'static {
        let log = Arc::clone(log);
        move |event, key| {
            log.lock()
                .unwrap()
                .push((event, key.to_str().unwrap().to_string()));
        }
    }

    #[test]
    fn b_the_observer_sees_successful_changes() {
        let log = Log::default();
        let mut table = Table::new(2).unwrap();
        table.observe(logging_observer(&log));
        table.put(c"a", b"1").unwrap();
        table.put(c"b", b"2").unwrap();
        table.put(c"a", b"3").unwrap();
        assert_eq!(table.put(c"c", b"4"), Err(Error::Full));
        table.remove(c"a").unwrap();
        assert_eq!(table.remove(c"zzz"), Err(Error::NotFound));
        assert_eq!(
            *log.lock().unwrap(),
            [
                (Event::Put, "a".to_string()),
                (Event::Put, "b".to_string()),
                (Event::Put, "a".to_string()),
                (Event::Remove, "a".to_string()),
            ],
            "one event per successful put and remove, none for a failed call"
        );
    }

    #[test]
    fn b_the_closure_is_dropped_when_the_table_is_dropped() {
        let log = Log::default();
        let mut table = Table::new(1).unwrap();
        table.observe(logging_observer(&log));
        assert_eq!(
            Arc::strong_count(&log),
            2,
            "the table must keep the closure (and what it captured) alive"
        );
        drop(table);
        assert_eq!(
            Arc::strong_count(&log),
            1,
            "dropping the table must drop the closure: pass a drop function \
             to kvtab_set_observer"
        );
    }

    #[test]
    fn b_replacing_the_observer_drops_the_old_closure() {
        let (first, second) = (Log::default(), Log::default());
        let mut table = Table::new(2).unwrap();
        table.observe(logging_observer(&first));
        table.put(c"a", b"1").unwrap();
        table.observe(logging_observer(&second));
        assert_eq!(
            Arc::strong_count(&first),
            1,
            "the replaced closure must be dropped"
        );
        table.put(c"b", b"2").unwrap();
        assert_eq!(first.lock().unwrap().len(), 1);
        assert_eq!(*second.lock().unwrap(), [(Event::Put, "b".to_string())]);
        drop(table);
        assert_eq!(Arc::strong_count(&second), 1);
    }

    #[test]
    fn b_clear_observer_drops_the_closure_and_stops_the_events() {
        let log = Log::default();
        let mut table = Table::new(2).unwrap();
        table.clear_observer();
        table.observe(logging_observer(&log));
        table.put(c"a", b"1").unwrap();
        table.clear_observer();
        assert_eq!(
            Arc::strong_count(&log),
            1,
            "clear_observer drops the closure"
        );
        table.put(c"b", b"2").unwrap();
        assert_eq!(
            log.lock().unwrap().len(),
            1,
            "no events after clear_observer"
        );
        table.clear_observer();
    }

    #[test]
    fn b_technique_the_closure_is_boxed_once() {
        let log = Log::default();
        let mut table = Table::new(1).unwrap();
        let observer = logging_observer(&log);
        let ((), allocations) = count(|| table.observe(observer));
        assert!(
            allocations <= 1,
            "observe made {allocations} allocations. Box the closure itself; \
             a generic trampoline knows its type, so no `Box<dyn FnMut>` is needed"
        );
    }

    #[test]
    fn b_the_observer_moves_to_another_thread_with_its_table() {
        let log = Log::default();
        let mut table = Table::new(2).unwrap();
        table.observe(logging_observer(&log));
        within(10, "the thread that received the table", move || {
            table.put(c"a", b"1").unwrap();
        });
        assert_eq!(*log.lock().unwrap(), [(Event::Put, "a".to_string())]);
        assert_eq!(
            Arc::strong_count(&log),
            1,
            "the table was dropped on that thread"
        );
    }

    #[test]
    fn p_predictions() {
        // The questions use the raw `sys` functions on a 64-bit target, so
        // they do not depend on your code.

        /// Counts its calls in the `usize` behind `userdata` and asks to
        /// stop at the second call.
        unsafe extern "C" fn stop_at_second(
            _key: *const c_char,
            _value: *const c_uchar,
            _value_len: usize,
            userdata: *mut c_void,
        ) -> c_int {
            // SAFETY: the test passes a pointer to a live usize.
            let calls = unsafe { &mut *userdata.cast::<usize>() };
            *calls += 1;
            c_int::from(*calls == 2)
        }

        // P1. A table with three entries is walked with `stop_at_second`.
        // Write (the status that kvtab_for_each returns, the number of calls).
        let mut calls: usize = 0;
        // SAFETY: the table is created, used and freed here; the keys are
        // NUL-terminated; `calls` outlives the walk.
        let status = unsafe {
            let table = sys::kvtab_new(3);
            for key in [c"a", c"b", c"c"] {
                sys::kvtab_put(table, key.as_ptr(), std::ptr::null(), 0);
            }
            let status = sys::kvtab_for_each(
                table,
                Some(stop_at_second),
                (&raw mut calls).cast::<c_void>(),
            );
            sys::kvtab_free(table);
            status
        };
        let p1: (c_int, usize) = guess();
        assert_eq!(p1, (status, calls), "P1: how a visitor stops a walk");

        // P2. `void *` is one pointer. Write the sizes in bytes of
        // `*mut c_void` and of `*mut dyn FnMut()`.
        let p2: (usize, usize) = guess();
        assert_eq!(
            p2,
            (size_of::<*mut c_void>(), size_of::<*mut dyn FnMut()>()),
            "P2: why a pointer to a `dyn FnMut` does not fit into userdata"
        );

        // P3. This function has the ABI "C-unwind", which allows a panic to
        // leave it. Does `catch_unwind` catch the panic? (With the ABI "C"
        // the process would abort instead.)
        extern "C-unwind" fn may_unwind() {
            panic!("P3: this panic is expected");
        }
        let caught = catch_unwind(|| may_unwind()).is_err();
        let p3: bool = guess();
        assert_eq!(p3, caught, "P3: C-unwind lets a panic pass");
    }
}
