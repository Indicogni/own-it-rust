//! pacer: how long to wait before the next retry.
//!
//! A program that retries a failed request should wait a little longer each
//! time. This crate computes the waiting times.
//!
//! ```
//! use pacer::Config;
//!
//! let config = Config { retries: 3, base_ms: 100 };
//! assert_eq!(pacer::delays(&config)?, [100, 200, 400]);
//! # Ok::<(), pacer::Error>(())
//! ```

use std::fmt;

/// How to retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// How often to retry. Must be at least 1.
    pub retries: u32,
    /// The delay before the first retry, in milliseconds. Each later delay is
    /// twice the one before it.
    pub base_ms: u64,
}

/// Why a [`Config`] cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `retries` is 0.
    NoRetries,
    /// A delay, or the sum of the delays, does not fit into a `u64`.
    Overflow,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoRetries => f.write_str("the number of retries is 0"),
            Error::Overflow => f.write_str("a delay is too large"),
        }
    }
}

impl std::error::Error for Error {}

/// The delay before each retry, in milliseconds: `base_ms`, then twice that,
/// and so on.
///
/// # Errors
///
/// [`Error::NoRetries`] if `config.retries` is 0, and [`Error::Overflow`] if a
/// delay does not fit into a `u64`.
///
/// ```
/// use pacer::{Config, Error};
///
/// let config = Config { retries: 4, base_ms: 50 };
/// assert_eq!(pacer::delays(&config)?, [50, 100, 200, 400]);
/// assert_eq!(pacer::delays(&Config { retries: 0, base_ms: 50 }), Err(Error::NoRetries));
/// # Ok::<(), Error>(())
/// ```
pub fn delays(config: &Config) -> Result<Vec<u64>, Error> {
    if config.retries == 0 {
        return Err(Error::NoRetries);
    }
    let mut delays = Vec::new();
    let mut delay = Some(config.base_ms);
    for _ in 0..config.retries {
        let current = delay.ok_or(Error::Overflow)?;
        delays.push(current);
        delay = current.checked_mul(2);
    }
    Ok(delays)
}

/// The sum of all delays, in milliseconds.
///
/// # Errors
///
/// As for [`delays`]; also [`Error::Overflow`] if the sum does not fit into a
/// `u64`.
///
/// ```
/// let config = pacer::Config { retries: 3, base_ms: 100 };
/// assert_eq!(pacer::get_total_ms(&config)?, 700);
/// # Ok::<(), pacer::Error>(())
/// ```
pub fn get_total_ms(config: &Config) -> Result<u64, Error> {
    delays(config)?
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
        .ok_or(Error::Overflow)
}

/// A source of time. Programs implement this trait for the system clock, and
/// tests implement it for a clock that they control.
pub trait Clock {
    /// The current time in milliseconds since some fixed moment.
    fn now_ms(&self) -> u64;
}

/// The time at which the last retry starts when the first attempt fails now.
///
/// # Errors
///
/// As for [`get_total_ms`]; also [`Error::Overflow`] if the time does not fit
/// into a `u64`.
///
/// ```
/// use pacer::{Clock, Config};
///
/// struct Fixed(u64);
///
/// impl Clock for Fixed {
///     fn now_ms(&self) -> u64 {
///         self.0
///     }
/// }
///
/// let config = Config { retries: 2, base_ms: 10 };
/// assert_eq!(pacer::deadline(&Fixed(1_000), &config)?, 1_030);
/// # Ok::<(), pacer::Error>(())
/// ```
pub fn deadline(clock: &impl Clock, config: &Config) -> Result<u64, Error> {
    clock
        .now_ms()
        .checked_add(get_total_ms(config)?)
        .ok_or(Error::Overflow)
}
