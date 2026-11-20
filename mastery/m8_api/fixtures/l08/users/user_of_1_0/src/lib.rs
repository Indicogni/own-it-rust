//! A program that was written against pacer 1.0.0. You do not need to edit
//! this file, and the capstone is passed only if you do not have to.
//!
//! It uses the library in every way that 1.0.0 allows: struct expressions and
//! patterns for `Config`, a `match` on `Error` without a wildcard, its own
//! implementation of `Clock`, function pointers, a glob import, and a trait of
//! its own on a type of the library.

use pacer::*;

/// A clock that the tests control.
pub struct TestClock {
    pub now: u64,
}

// Written for 1.0.0, where `Clock` has one method.
impl Clock for TestClock {
    fn now_ms(&self) -> u64 {
        self.now
    }
}

/// This program's own summary of a configuration.
pub trait Summary {
    /// The total waiting time, or `None` if the configuration is not usable.
    fn total_ms(&self) -> Option<u64>;
}

impl Summary for Config {
    fn total_ms(&self) -> Option<u64> {
        get_total_ms(self).ok()
    }
}

/// A function of this program that happens to have a common name. A name
/// that this file defines wins over a name that a glob import brings in.
pub fn total_ms(delays: &[u64]) -> u64 {
    delays.iter().sum()
}

/// What this program tells its user. No wildcard arm: in 1.0.0 the two
/// variants are all there is.
pub fn explain(error: Error) -> &'static str {
    match error {
        Error::NoRetries => "set retries to 1 or more",
        Error::Overflow => "use a smaller base delay",
    }
}

/// The default of this program, written as a struct expression.
pub fn default_config() -> Config {
    Config {
        retries: 3,
        base_ms: 100,
    }
}

/// A slower variant of a configuration, taken apart with a full pattern.
pub fn slower(config: Config) -> Config {
    let Config { retries, base_ms } = config;
    Config {
        retries,
        base_ms: base_ms * 10,
    }
}

/// Compiles only for types that can be used as a boxed, thread-safe error.
pub fn portable_error<T: std::error::Error + Send + Sync + 'static>() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delays_double() {
        assert_eq!(delays(&default_config()), Ok(vec![100, 200, 400]));
        assert_eq!(
            delays(&slower(default_config())),
            Ok(vec![1000, 2000, 4000])
        );
        let none = Config {
            retries: 0,
            base_ms: 1,
        };
        assert_eq!(delays(&none), Err(Error::NoRetries));
        let huge = Config {
            retries: 2,
            base_ms: u64::MAX,
        };
        assert_eq!(delays(&huge), Err(Error::Overflow));
    }

    #[test]
    fn totals_and_deadlines() {
        assert_eq!(get_total_ms(&default_config()), Ok(700));
        assert_eq!(
            deadline(&TestClock { now: 5_000 }, &default_config()),
            Ok(5_700)
        );
        assert_eq!(
            deadline(&TestClock { now: u64::MAX }, &default_config()),
            Err(Error::Overflow)
        );
    }

    #[test]
    fn functions_keep_their_types() {
        // A function pointer type states the whole signature.
        let list: fn(&Config) -> Result<Vec<u64>, Error> = delays;
        let sum: fn(&Config) -> Result<u64, Error> = get_total_ms;
        assert_eq!(list(&default_config()).map(|delays| delays.len()), Ok(3));
        assert_eq!(sum(&default_config()), Ok(700));
    }

    #[test]
    fn this_programs_own_names_still_mean_the_same() {
        // The trait method of this program, called with method syntax.
        let total: Option<u64> = default_config().total_ms();
        assert_eq!(total, Some(700));
        // The function of this program, although `pacer::*` is imported.
        assert_eq!(total_ms(&[1, 2, 3]), 6);
    }

    #[test]
    fn values_are_plain_data() {
        let config = default_config();
        let copy = config;
        assert_eq!(config, copy, "Config is Copy and comparable");
        assert_eq!(std::mem::size_of::<Config>(), 16, "Config is two numbers");
        let error = Error::Overflow;
        let copy = error;
        assert_eq!(explain(error), "use a smaller base delay");
        assert_eq!(copy.to_string(), "a delay is too large");
        portable_error::<Error>();
    }
}
