//! A program that is waiting for the next release of pacer. You do not need
//! to edit this file. Together with the lesson header it is the contract of
//! the new interface: it names every new item, and nothing else.

// A project that turns warnings into errors: the new interface must be usable
// without touching anything deprecated.
#![deny(warnings)]

use pacer::{Clock, Delays, PolicyError, PolicyErrorKind};

/// A clock that the tests control.
pub struct TestClock(pub u64);

impl Clock for TestClock {
    fn now_ms(&self) -> u64 {
        self.0
    }
}

/// How this program reacts to a policy that cannot be built. The wildcard arm
/// is required: the library may add kinds.
pub fn explain(error: &PolicyError) -> &'static str {
    match error.kind() {
        PolicyErrorKind::ZeroRetries => "set retries to 1 or more",
        PolicyErrorKind::ZeroFactor => "set the factor to 1 or more",
        _ => "see the documentation of pacer",
    }
}

/// The retries that this program still has to make. The iterator of the
/// library is a field, so its type must have a name: an `impl Iterator` return
/// type could not be written here.
#[derive(Debug, Clone)]
pub struct Schedule {
    remaining: Delays,
}

impl Schedule {
    /// The schedule of a policy, before the first retry.
    pub fn new(policy: &pacer::Policy) -> Schedule {
        Schedule {
            remaining: policy.delays(),
        }
    }

    /// How long to wait before the next retry, or `None` when no retry is left.
    pub fn next_delay_ms(&mut self) -> Option<u64> {
        self.remaining.next()
    }
}

/// Compiles only for types that can be used as a boxed, thread-safe error.
pub fn portable_error<T: std::error::Error + Send + Sync + 'static>() {}

/// Compiles only for types that can be moved to and shared with other threads.
pub fn thread_safe<T: Send + Sync>() {}

/// Counts the heap allocations of the current thread, so that a test can check
/// that an operation makes none.
#[cfg(test)]
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    /// Forwards to the system allocator and counts allocations and growth.
    pub struct Counting;

    fn record() {
        // `try_with`: during the teardown of a thread the counter may be gone.
        let _ = COUNT.try_with(|count| count.set(count.get() + 1));
    }

    // SAFETY: every method forwards its arguments unchanged to `System`, which
    // upholds the contract of `GlobalAlloc`. `record` touches only a
    // thread-local that is initialised by a constant and has no destructor, so
    // it never allocates and never calls back into this allocator.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds the contract of `GlobalAlloc::alloc`,
            // which is forwarded unchanged.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record();
            // SAFETY: the caller upholds the contract of
            // `GlobalAlloc::alloc_zeroed`, which is forwarded unchanged.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record();
            // SAFETY: the caller upholds the contract of `GlobalAlloc::realloc`:
            // `ptr` came from this allocator, which means from `System`, with
            // `layout`. All arguments are forwarded unchanged.
            unsafe { System.realloc(ptr, layout, new_size) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: the caller upholds the contract of `GlobalAlloc::dealloc`:
            // `ptr` came from this allocator, which means from `System`, with
            // `layout`. Both are forwarded unchanged.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    #[global_allocator]
    static ALLOCATOR: Counting = Counting;

    /// Runs `f` and returns its result with the number of allocations that it
    /// made on this thread.
    pub fn allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let result = f();
        (result, COUNT.with(Cell::get) - before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pacer::{Config, Policy};

    fn list(policy: &Policy) -> Vec<u64> {
        policy.delays().collect()
    }

    #[test]
    fn a_new_policy_doubles_like_a_config() -> Result<(), PolicyError> {
        let policy = Policy::new(4, 100)?;
        assert_eq!(list(&policy), [100, 200, 400, 800]);
        assert_eq!(policy.total_ms(), 1500);
        assert_eq!(
            (
                policy.retries(),
                policy.base_ms(),
                policy.factor(),
                policy.cap_ms()
            ),
            (4, 100, 2, None)
        );
        Ok(())
    }

    #[test]
    fn the_factor_and_the_cap_are_settings() -> Result<(), PolicyError> {
        let tripled = Policy::new(4, 10)?.with_factor(3)?;
        assert_eq!(list(&tripled), [10, 30, 90, 270]);
        assert_eq!(tripled.factor(), 3);

        let constant = Policy::new(3, 250)?.with_factor(1)?;
        assert_eq!(list(&constant), [250, 250, 250]);

        let capped = Policy::new(5, 100)?.with_cap_ms(350);
        assert_eq!(list(&capped), [100, 200, 350, 350, 350]);
        assert_eq!(capped.cap_ms(), Some(350));
        assert_eq!(capped.total_ms(), 1350);

        // The settings can be combined in any order.
        let both = Policy::new(4, 5)?.with_cap_ms(100).with_factor(10)?;
        assert_eq!(list(&both), [5, 50, 100, 100]);

        // A cap below the base delay is allowed: every delay is the cap.
        assert_eq!(list(&Policy::new(2, 100)?.with_cap_ms(30)), [30, 30]);
        Ok(())
    }

    #[test]
    fn a_policy_never_overflows() -> Result<(), PolicyError> {
        // Without a cap, a delay that does not fit is the largest u64.
        let huge = Policy::new(3, u64::MAX / 2 + 1)?;
        assert_eq!(list(&huge), [u64::MAX / 2 + 1, u64::MAX, u64::MAX]);
        assert_eq!(huge.total_ms(), u64::MAX);
        // With a cap, it is the cap.
        assert_eq!(list(&huge.with_cap_ms(7)), [7, 7, 7]);
        Ok(())
    }

    #[test]
    fn delays_are_computed_one_at_a_time() -> Result<(), PolicyError> {
        let patient = Policy::new(1_000_000, 1)?.with_cap_ms(8);
        let (first, allocations) = counting::allocations(|| {
            let mut delays = patient.delays();
            [
                delays.next(),
                delays.next(),
                delays.next(),
                delays.next(),
                delays.next(),
            ]
        });
        assert_eq!(first, [Some(1), Some(2), Some(4), Some(8), Some(8)]);
        assert_eq!(
            allocations, 0,
            "`delays` must not allocate: the iterator computes each delay when it is \
             asked for, and does not build a collection of a million numbers first"
        );
        // Every call starts again at the first delay.
        assert_eq!(patient.delays().next(), Some(1));
        Ok(())
    }

    #[test]
    fn invalid_policies_are_refused_with_a_kind() {
        let no_retries = Policy::new(0, 100).expect_err("0 retries is not a policy");
        assert_eq!(no_retries.kind(), PolicyErrorKind::ZeroRetries);
        assert_eq!(explain(&no_retries), "set retries to 1 or more");

        let no_factor = Policy::new(1, 100)
            .expect("1 retry is a policy")
            .with_factor(0)
            .expect_err("the factor 0 is refused");
        assert_eq!(no_factor.kind(), PolicyErrorKind::ZeroFactor);

        for error in [no_retries, no_factor] {
            let text = error.to_string();
            assert!(
                !text.is_empty() && !text.ends_with('.'),
                "bad message: {text:?}"
            );
            assert!(!format!("{error:?}").is_empty());
        }
        portable_error::<PolicyError>();
    }

    #[test]
    fn a_config_converts_to_a_policy() {
        let config = Config {
            retries: 3,
            base_ms: 100,
        };
        let policy = Policy::try_from(config).expect("3 retries is a policy");
        assert_eq!(
            list(&policy),
            pacer::delays(&config).expect("a usable config")
        );
        assert_eq!(Ok(policy), Policy::new(3, 100), "policies can be compared");

        let invalid = Config {
            retries: 0,
            base_ms: 100,
        };
        let error = Policy::try_from(invalid).expect_err("0 retries is not a policy");
        assert_eq!(error.kind(), PolicyErrorKind::ZeroRetries);
    }

    #[test]
    fn the_new_names_are_there() {
        let config = Config {
            retries: 3,
            base_ms: 100,
        };
        // The free function with the better name, for a `Config`.
        assert_eq!(pacer::total_ms(&config), Ok(700));
        // A method that every clock has now.
        assert_eq!(TestClock(1_250).elapsed_since(1_000), 250);
        assert_eq!(TestClock(1_250).elapsed_since(9_000), 0);
    }

    #[test]
    fn the_iterator_has_a_name_and_can_be_stored() -> Result<(), PolicyError> {
        let mut schedule = Schedule::new(&Policy::new(2, 40)?);
        let later = schedule.clone();
        assert_eq!(schedule.next_delay_ms(), Some(40));
        assert_eq!(schedule.next_delay_ms(), Some(80));
        assert_eq!(schedule.next_delay_ms(), None);
        // The clone was taken before the first step and starts from there.
        assert_eq!(later.remaining.collect::<Vec<u64>>(), [40, 80]);
        Ok(())
    }

    #[test]
    fn policies_are_plain_values() -> Result<(), PolicyError> {
        let policy = Policy::new(2, 10)?;
        let copy = policy;
        assert_eq!(policy, copy, "Policy is Copy and comparable");
        assert!(format!("{policy:?}").contains("Policy"));
        let delays = policy.delays();
        assert_eq!(delays.clone().count(), 2, "the iterator can be cloned");
        assert!(!format!("{delays:?}").is_empty());
        thread_safe::<Policy>();
        thread_safe::<PolicyError>();
        Ok(())
    }
}
