//! Helpers shared by the acceptance tests. Nothing in this file is part of
//! the API under test, and you never need to change it.
//!
//! | Helper | Purpose |
//! | --- | --- |
//! | [`bounded`] | Runs a test body with a time limit, so that a loop that never ends becomes a failure and not a hang. |
//! | [`Ledger`], [`Tracked`] | Elements that record how often each of them was dropped. |
//! | [`expect_panic`] | Runs code that must panic and returns the panic message. |
//! | [`Rng`] | A small deterministic random number generator for the model tests. |
//! | [`lies_inside`] | Tells whether a value's bytes are stored inside another value. |

// Every test binary includes this file and uses a different part of it.
#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

/// The time limit of one test body, in seconds.
pub const LIMIT_SECS: u64 = 20;

/// Runs `body` on a helper thread and returns its result. If `body` has not
/// finished after [`LIMIT_SECS`] seconds, the test fails with a message
/// instead of hanging.
///
/// A panic inside `body` (a failed assertion, or a panic that a
/// `#[should_panic]` test expects) continues in the calling test with the
/// same message.
///
/// Under Miri the body runs directly on the test thread: Miri is much slower
/// than a normal run and measures time differently, so a limit would only
/// produce false alarms.
#[track_caller]
pub fn bounded<R: Send + 'static>(body: impl FnOnce() -> R + Send + 'static) -> R {
    if cfg!(miri) {
        return body();
    }
    use std::sync::mpsc::{self, RecvTimeoutError};
    let name = std::thread::current()
        .name()
        .unwrap_or("test body")
        .to_string();
    let (sender, receiver) = mpsc::channel();
    let helper = std::thread::Builder::new()
        .name(name)
        .spawn(move || {
            let _ = sender.send(body());
        })
        .expect("the operating system refused to start a helper thread");
    match receiver.recv_timeout(std::time::Duration::from_secs(LIMIT_SECS)) {
        Ok(value) => value,
        Err(RecvTimeoutError::Timeout) => panic!(
            "the test was still running after {LIMIT_SECS} s. A loop in the code under test \
             probably never ends: look for an iterator that never returns None, or a length \
             that is never reduced"
        ),
        Err(RecvTimeoutError::Disconnected) => match helper.join() {
            Err(payload) => panic::resume_unwind(payload),
            Ok(()) => unreachable!("the helper thread ended without a result and without a panic"),
        },
    }
}

/// Runs `code`, which must panic with a message that contains `expected`,
/// and returns the whole message. `what` describes the call for the failure
/// message.
///
/// The check of the message matters: without it, the `todo!()` of an
/// unfinished function would count as the expected panic.
#[track_caller]
pub fn expect_panic<R>(what: &str, expected: &str, code: impl FnOnce() -> R) -> String {
    let outcome = panic::catch_unwind(AssertUnwindSafe(code));
    let payload = match outcome {
        Ok(_) => panic!("{what} must panic, but it returned normally"),
        Err(payload) => payload,
    };
    let message = if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        String::from("(a panic payload that is not a string)")
    };
    assert!(
        message.contains(expected),
        "{what} panicked, but with the wrong message.\n  expected a message containing: \
         {expected:?}\n  the message was: {message:?}"
    );
    message
}

/// Records, for every [`Tracked`] value made from it, how often that value
/// was dropped.
///
/// A test creates one ledger first (so that it outlives the collection),
/// fills a collection with `ledger.item(tag)` values, and asks the ledger
/// afterwards what happened to them.
pub struct Ledger {
    /// `drops[serial]` is the number of times the value with that serial
    /// number was dropped. A correct collection never makes it exceed 1.
    drops: RefCell<Vec<u32>>,
    /// The tags of the dropped values, in the order of the drops.
    order: RefCell<Vec<u32>>,
    /// Counts down to the clone that panics.
    clone_fuse: Cell<Option<u32>>,
    /// The tag whose next drop panics.
    drop_bomb: Cell<Option<u32>>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            drops: RefCell::new(Vec::new()),
            order: RefCell::new(Vec::new()),
            clone_fuse: Cell::new(None),
            drop_bomb: Cell::new(None),
        }
    }

    /// Creates a tracked value. `tag` is what the value looks like to the
    /// test: two values with the same tag compare equal.
    pub fn item(&self, tag: u32) -> Tracked<'_> {
        let mut drops = self.drops.borrow_mut();
        drops.push(0);
        Tracked {
            tag,
            serial: drops.len() - 1,
            ledger: self,
        }
    }

    /// Creates one tracked value per tag.
    pub fn items(&self, tags: impl IntoIterator<Item = u32>) -> Vec<Tracked<'_>> {
        tags.into_iter().map(|tag| self.item(tag)).collect()
    }

    /// The number of values created so far (clones included).
    pub fn created(&self) -> usize {
        self.drops.borrow().len()
    }

    /// The number of values that were created and never dropped.
    pub fn live(&self) -> usize {
        self.drops
            .borrow()
            .iter()
            .filter(|&&count| count == 0)
            .count()
    }

    /// The tags of all dropped values, in the order in which they were
    /// dropped.
    pub fn dropped(&self) -> Vec<u32> {
        self.order.borrow().clone()
    }

    /// Makes the `nth` call of `Tracked::clone` from now on panic (1 is the
    /// next call).
    pub fn panic_on_clone(&self, nth: u32) {
        assert!(nth >= 1, "clones are counted from 1");
        self.clone_fuse.set(Some(nth));
    }

    /// Makes the destructor of the next dropped value with this tag panic.
    /// The drop is recorded first.
    pub fn panic_on_drop_of(&self, tag: u32) {
        self.drop_bomb.set(Some(tag));
    }

    /// Fails if any value was dropped more than once. `when` describes the
    /// moment for the failure message.
    #[track_caller]
    pub fn assert_no_double_drop(&self, when: &str) {
        let twice = self.drops.borrow().iter().filter(|&&n| n > 1).count();
        assert!(
            twice == 0,
            "{when}: {twice} element(s) were dropped more than once. A double drop is undefined \
             behaviour for real types: some slot was read as a value after its value had been \
             moved out or dropped. Drop order so far (tags): {:?}",
            self.dropped()
        );
    }

    /// Fails unless every value created so far was dropped exactly once.
    #[track_caller]
    pub fn assert_all_dropped_once(&self, when: &str) {
        self.assert_no_double_drop(when);
        let live = self.live();
        assert!(
            live == 0,
            "{when}: {live} of {} element(s) were never dropped (leaked). Drop order so far \
             (tags): {:?}",
            self.created(),
            self.dropped()
        );
    }
}

/// An element that reports its drop to a [`Ledger`].
///
/// It compares and prints by its tag only. It owns no heap memory on
/// purpose: if a faulty collection drops it twice, the ledger notices and the
/// test fails with a message, where a `String` would crash the test program.
pub struct Tracked<'a> {
    pub tag: u32,
    serial: usize,
    ledger: &'a Ledger,
}

impl Drop for Tracked<'_> {
    fn drop(&mut self) {
        self.ledger.drops.borrow_mut()[self.serial] += 1;
        self.ledger.order.borrow_mut().push(self.tag);
        if self.ledger.drop_bomb.get() == Some(self.tag) {
            self.ledger.drop_bomb.set(None);
            // A second panic during unwinding would abort the test program.
            if !std::thread::panicking() {
                panic!(
                    "drop bomb: the destructor of element {} panics on purpose",
                    self.tag
                );
            }
        }
    }
}

impl Clone for Tracked<'_> {
    fn clone(&self) -> Self {
        if let Some(left) = self.ledger.clone_fuse.get() {
            if left == 1 {
                self.ledger.clone_fuse.set(None);
                panic!("clone bomb: cloning element {} panics on purpose", self.tag);
            }
            self.ledger.clone_fuse.set(Some(left - 1));
        }
        self.ledger.item(self.tag)
    }
}

impl PartialEq for Tracked<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag
    }
}

impl Eq for Tracked<'_> {}

impl fmt::Debug for Tracked<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tag)
    }
}

/// The tags of a sequence of tracked values.
pub fn tags<'a, 'b: 'a>(items: impl IntoIterator<Item = &'a Tracked<'b>>) -> Vec<u32> {
    items.into_iter().map(|item| item.tag).collect()
}

/// A deterministic pseudo-random number generator (xorshift64*). The model
/// tests use a fixed seed, so every run makes the same sequence of calls.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..bound`. `bound` must not be 0.
    pub fn below(&mut self, bound: usize) -> usize {
        ((self.next() >> 33) % bound as u64) as usize
    }
}

/// `true` if every byte of `inner` is stored inside the bytes of `outer`.
///
/// The tests use it to show that a collection keeps its elements inside
/// itself (inline), and not somewhere else. `inner` must not be zero-sized.
pub fn lies_inside<I: ?Sized, O: ?Sized>(inner: &I, outer: &O) -> bool {
    let inner_start = std::ptr::from_ref(inner).cast::<u8>() as usize;
    let inner_end = inner_start + size_of_val(inner);
    let outer_start = std::ptr::from_ref(outer).cast::<u8>() as usize;
    let outer_end = outer_start + size_of_val(outer);
    assert!(
        inner_start != inner_end,
        "lies_inside: a zero-sized value has no bytes to locate"
    );
    outer_start <= inner_start && inner_end <= outer_end
}

/// The number of steps of a model test: fewer under Miri, which is slow.
pub fn steps(normal: usize, under_miri: usize) -> usize {
    if cfg!(miri) { under_miri } else { normal }
}
