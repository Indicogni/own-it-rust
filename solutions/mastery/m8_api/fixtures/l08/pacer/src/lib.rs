//! pacer: how long to wait before the next retry.
//!
//! A program that retries a failed request should wait a little longer each
//! time. This crate computes the waiting times.
//!
//! Since version 1.1.0 the main type is [`Policy`]:
//!
//! ```
//! use pacer::Policy;
//!
//! let policy = Policy::new(4, 100)?.with_cap_ms(250);
//! let delays: Vec<u64> = policy.delays().collect();
//! assert_eq!(delays, [100, 200, 250, 250]);
//! # Ok::<(), pacer::PolicyError>(())
//! ```
//!
//! The functions of version 1.0.0 work on a [`Config`] and are still here:
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
/// assert_eq!(pacer::total_ms(&config)?, 700);
/// # Ok::<(), pacer::Error>(())
/// ```
pub fn total_ms(config: &Config) -> Result<u64, Error> {
    delays(config)?
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
        .ok_or(Error::Overflow)
}

/// The sum of all delays, in milliseconds. The old name of [`total_ms`].
///
/// # Errors
///
/// As for [`total_ms`].
// A wrapper and not a second copy, so that the two names cannot drift apart.
#[deprecated(since = "1.1.0", note = "use `total_ms` instead")]
pub fn get_total_ms(config: &Config) -> Result<u64, Error> {
    total_ms(config)
}

/// A source of time. Programs implement this trait for the system clock, and
/// tests implement it for a clock that they control.
pub trait Clock {
    /// The current time in milliseconds since some fixed moment.
    fn now_ms(&self) -> u64;

    /// The milliseconds from `start_ms` to now; 0 if `start_ms` is later.
    ///
    /// This method has a body, so that the implementations of `Clock` that
    /// were written for version 1.0.0 still compile.
    ///
    /// ```
    /// use pacer::Clock;
    ///
    /// struct Fixed(u64);
    ///
    /// impl Clock for Fixed {
    ///     fn now_ms(&self) -> u64 {
    ///         self.0
    ///     }
    /// }
    ///
    /// assert_eq!(Fixed(1_250).elapsed_since(1_000), 250);
    /// assert_eq!(Fixed(1_250).elapsed_since(2_000), 0);
    /// ```
    fn elapsed_since(&self, start_ms: u64) -> u64 {
        self.now_ms().saturating_sub(start_ms)
    }
}

/// The time at which the last retry starts when the first attempt fails now.
///
/// # Errors
///
/// As for [`total_ms`]; also [`Error::Overflow`] if the time does not fit
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
        .checked_add(total_ms(config)?)
        .ok_or(Error::Overflow)
}

/// A retry policy: how often to retry, and how the delays grow.
///
/// The fields are private. A policy is built with [`Policy::new`] and the
/// `with_` methods, so every `Policy` value is valid, and later versions can
/// add settings without breaking anybody.
///
/// ```
/// use pacer::Policy;
///
/// // Three retries: 100 ms, then three times as long each time, at most 500 ms.
/// let policy = Policy::new(3, 100)?.with_factor(3)?.with_cap_ms(500);
/// assert_eq!(policy.delays().collect::<Vec<u64>>(), [100, 300, 500]);
/// assert_eq!(policy.total_ms(), 900);
/// # Ok::<(), pacer::PolicyError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    retries: u32,
    base_ms: u64,
    factor: u32,
    cap_ms: Option<u64>,
}

impl Policy {
    /// A policy with `retries` retries. The first delay is `base_ms`, and each
    /// later delay is twice the one before it.
    ///
    /// # Errors
    ///
    /// A [`PolicyError`] of the kind [`PolicyErrorKind::ZeroRetries`] if
    /// `retries` is 0.
    pub fn new(retries: u32, base_ms: u64) -> Result<Policy, PolicyError> {
        if retries == 0 {
            return Err(PolicyError {
                kind: PolicyErrorKind::ZeroRetries,
            });
        }
        Ok(Policy {
            retries,
            base_ms,
            factor: 2,
            cap_ms: None,
        })
    }

    /// The same policy with another growth factor: each delay is `factor`
    /// times the one before it. With the factor 1 all delays are equal.
    ///
    /// # Errors
    ///
    /// A [`PolicyError`] of the kind [`PolicyErrorKind::ZeroFactor`] if
    /// `factor` is 0.
    pub fn with_factor(self, factor: u32) -> Result<Policy, PolicyError> {
        if factor == 0 {
            return Err(PolicyError {
                kind: PolicyErrorKind::ZeroFactor,
            });
        }
        Ok(Policy { factor, ..self })
    }

    /// The same policy with a largest delay: no delay is longer than `cap_ms`.
    pub fn with_cap_ms(self, cap_ms: u64) -> Policy {
        Policy {
            cap_ms: Some(cap_ms),
            ..self
        }
    }

    /// How often to retry.
    pub fn retries(&self) -> u32 {
        self.retries
    }

    /// The delay before the first retry, in milliseconds.
    pub fn base_ms(&self) -> u64 {
        self.base_ms
    }

    /// The growth factor.
    pub fn factor(&self) -> u32 {
        self.factor
    }

    /// The largest delay, if one was set.
    pub fn cap_ms(&self) -> Option<u64> {
        self.cap_ms
    }

    /// The delay before each retry, in milliseconds.
    ///
    /// A delay that would not fit into a `u64` is `u64::MAX`, or the cap if
    /// there is one. This method cannot fail.
    pub fn delays(&self) -> Delays {
        Delays {
            next: self.base_ms,
            remaining: self.retries,
            factor: self.factor,
            cap_ms: self.cap_ms,
        }
    }

    /// The sum of all delays, in milliseconds; `u64::MAX` if the sum does not
    /// fit into a `u64`.
    pub fn total_ms(&self) -> u64 {
        self.delays().fold(0, u64::saturating_add)
    }
}

/// A [`Config`] is a policy with the factor 2 and no cap.
impl TryFrom<Config> for Policy {
    type Error = PolicyError;

    fn try_from(config: Config) -> Result<Policy, PolicyError> {
        Policy::new(config.retries, config.base_ms)
    }
}

/// The delays of a [`Policy`], in milliseconds. See [`Policy::delays`].
///
/// A type of this crate and not `Vec<u64>`: the delays are computed one at a
/// time, and the way they are computed can change.
#[derive(Debug, Clone)]
pub struct Delays {
    next: u64,
    remaining: u32,
    factor: u32,
    cap_ms: Option<u64>,
}

impl Iterator for Delays {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        self.remaining = self.remaining.checked_sub(1)?;
        let delay = self.cap_ms.map_or(self.next, |cap| self.next.min(cap));
        self.next = self.next.saturating_mul(u64::from(self.factor));
        Some(delay)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // A `u32` fits into a `usize` on every platform that this crate supports;
        // if it ever does not, the lower bound is still correct.
        match usize::try_from(self.remaining) {
            Ok(remaining) => (remaining, Some(remaining)),
            Err(_) => (usize::MAX, None),
        }
    }
}

/// Why a [`Policy`] cannot be built.
///
/// A struct with a [`kind`](PolicyError::kind), so that later versions can
/// carry more information without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError {
    kind: PolicyErrorKind,
}

impl PolicyError {
    /// What is wrong.
    pub fn kind(&self) -> PolicyErrorKind {
        self.kind
    }
}

/// The kinds of [`PolicyError`]. More kinds may be added in a minor release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PolicyErrorKind {
    /// The number of retries is 0.
    ZeroRetries,
    /// The growth factor is 0.
    ZeroFactor,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            PolicyErrorKind::ZeroRetries => f.write_str("the number of retries is 0"),
            PolicyErrorKind::ZeroFactor => f.write_str("the growth factor is 0"),
        }
    }
}

impl std::error::Error for PolicyError {}
