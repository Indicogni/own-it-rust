//! Milestone m14 (Chapter 14): a filter language compiled to closures.
//!
//! A filter is a list of terms separated by whitespace. A record matches
//! when it matches *every* term; an empty filter matches everything.
//!
//! ```text
//! level>=warn target=auth user!=bob latency_ms>100 msg~timeout ts>=2026-10-23T17:00:00Z
//! ```
//!
//! A term is `KEY OP VALUE`. The operator is found at the first byte of the
//! term that is one of `= ! < > ~`; the operator is then `!=`, `<=`, `>=`
//! (two bytes) or `=`, `<`, `>`, `~` (one byte). The key is everything before
//! it; the value is everything after it and may be empty (`user=`).
//!
//! | Key | Operators | Meaning |
//! | --- | --- | --- |
//! | `level` | `= != < <= > >=` | compare with a level name (any case, via `FromStr`) |
//! | `target` | `= != ~` | compare with the target; `~` means "contains" |
//! | `ts` | `= != < <= > >=` | compare timestamps as strings; the value must pass `is_timestamp` |
//! | any other key | `= !=` | the first field with that key equals / does not equal the value; a missing field never equals |
//! | any other key | `~` | the field exists and contains the value |
//! | any other key | `< <= > >=` | the field's `number()` is `Ok(Some(n))` and `n OP value`; the value must be a number (`parse_u64`, `Overflow::Reject`) |
//!
//! So `user!=bob` matches records without a `user` field, and
//! `latency_ms>100` never matches a record whose latency is missing or not a
//! number.
//!
//! Parse once, match many times: [`Filter::parse`] validates every term and
//! turns it into a boxed closure that owns the values it needs. Matching
//! performs no parsing of the filter and no allocation.
//!
//! One exception arrives with the m17 refactor, when `=`, `!=`, and `~` on a
//! field compare the *decoded* text: a quoted value that contains a
//! backslash escape must be decoded into a new `String` before it can be
//! compared, which is one allocation for that record. Bare values and
//! quoted values without escapes are still compared in place.

use crate::level::Level;
use crate::num::{Overflow, parse_u64};
use crate::record::{Record, is_timestamp};
use std::{fmt, str::FromStr};

/// One compiled term. `Send + Sync` lets threads share a filter (m19).
type Predicate = Box<dyn Fn(&Record<'_>) -> bool + Send + Sync>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
}

fn holds<T: Ord + ?Sized>(op: Op, left: &T, right: &T) -> bool {
    match op {
        Op::Eq => left == right,
        Op::Ne => left != right,
        Op::Lt => left < right,
        Op::Le => left <= right,
        Op::Gt => left > right,
        Op::Ge => left >= right,
        Op::Contains => false,
    }
}

fn compile(term: &str) -> Result<Predicate, FilterError> {
    let fail = |reason| FilterError {
        term: term.to_string(),
        reason,
    };
    let at = term
        .find(['=', '!', '<', '>', '~'])
        .ok_or_else(|| fail("expected KEY OP VALUE"))?;
    let (op, len) = match &term.as_bytes()[at..] {
        [b'!', b'=', ..] => (Op::Ne, 2),
        [b'<', b'=', ..] => (Op::Le, 2),
        [b'>', b'=', ..] => (Op::Ge, 2),
        [b'=', ..] => (Op::Eq, 1),
        [b'<', ..] => (Op::Lt, 1),
        [b'>', ..] => (Op::Gt, 1),
        [b'~', ..] => (Op::Contains, 1),
        _ => return Err(fail("expected KEY OP VALUE")),
    };
    let key = &term[..at];
    let value = &term[at + len..];
    if key.is_empty() {
        return Err(fail("empty key"));
    }
    let ordering = matches!(op, Op::Lt | Op::Le | Op::Gt | Op::Ge);
    match key {
        "level" => {
            if op == Op::Contains {
                return Err(fail("unsupported operator"));
            }
            let wanted: Level = value.parse().map_err(|_| fail("unknown level"))?;
            Ok(Box::new(move |r: &Record<'_>| holds(op, &r.level, &wanted)))
        }
        "target" => {
            if ordering {
                return Err(fail("unsupported operator"));
            }
            let value = value.to_string();
            Ok(Box::new(move |r: &Record<'_>| match op {
                Op::Contains => r.target.contains(value.as_str()),
                _ => holds(op, r.target, value.as_str()),
            }))
        }
        "ts" => {
            if op == Op::Contains {
                return Err(fail("unsupported operator"));
            }
            if !is_timestamp(value) {
                return Err(fail("expected a timestamp"));
            }
            let value = value.to_string();
            Ok(Box::new(move |r: &Record<'_>| {
                holds(op, r.timestamp, value.as_str())
            }))
        }
        _ if ordering => {
            let wanted =
                parse_u64(value, Overflow::Reject).map_err(|_| fail("expected a number"))?;
            let key = key.to_string();
            Ok(Box::new(
                move |r: &Record<'_>| matches!(r.number(&key), Ok(Some(n)) if holds(op, &n, &wanted)),
            ))
        }
        _ => {
            let key = key.to_string();
            let value = value.to_string();
            Ok(Box::new(move |r: &Record<'_>| {
                let found = r.text(&key);
                match op {
                    Op::Eq => found.is_some_and(|t| *t == *value),
                    Op::Ne => found.is_none_or(|t| *t != *value),
                    _ => found.is_some_and(|t| t.contains(value.as_str())),
                }
            }))
        }
    }
}

/// A compiled filter.
pub struct Filter {
    source: String,
    predicates: Vec<Predicate>,
}

/// Why a filter term was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterError {
    /// The whole offending term, such as `"level>=loud"`.
    pub term: String,
    /// One of the reasons listed on [`Filter::parse`].
    pub reason: &'static str,
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "term {:?}: {}", self.term, self.reason)
    }
}

impl std::error::Error for FilterError {}

impl Filter {
    /// m14 A. Compile a filter (grammar in the module documentation).
    ///
    /// Reject the first invalid term with one of these exact reasons:
    ///
    /// | Problem | `reason` |
    /// | --- | --- |
    /// | no operator, or `!` not followed by `=` | `"expected KEY OP VALUE"` |
    /// | the key is empty | `"empty key"` |
    /// | `level` value is not a level | `"unknown level"` |
    /// | `ts` value fails `is_timestamp` | `"expected a timestamp"` |
    /// | `<`, `<=`, `>`, `>=` value of a field term is not a number | `"expected a number"` |
    /// | operator not allowed for this key (e.g. `level~x`, `target>a`) | `"unsupported operator"` |
    ///
    /// Check in this order: operator, key, operator allowed for the key, value.
    /// So `level~loud` is an unsupported operator, not an unknown level.
    ///
    /// Each term becomes one boxed predicate: a `move` closure that captures
    /// owned copies of what it compares against (a `Level`, a `String`, a
    /// `u64`). Write a small private helper per key kind rather than one
    /// giant closure.
    pub fn parse(source: &str) -> Result<Filter, FilterError> {
        let predicates = source
            .split_ascii_whitespace()
            .map(compile)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Filter {
            source: source.to_string(),
            predicates,
        })
    }

    /// m14 B. Whether `record` satisfies every term.
    pub fn matches(&self, record: &Record<'_>) -> bool {
        self.predicates.iter().all(|p| p(record))
    }

    /// The text the filter was parsed from, unchanged.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// A filter that matches every record.
    pub fn all() -> Filter {
        Filter {
            source: String::new(),
            predicates: Vec::new(),
        }
    }
}

impl Default for Filter {
    fn default() -> Self {
        Filter::all()
    }
}

impl fmt::Debug for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Filter")
            .field("source", &self.source)
            .field("terms", &self.predicates.len())
            .finish()
    }
}

impl FromStr for Filter {
    type Err = FilterError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Filter::parse(s)
    }
}
