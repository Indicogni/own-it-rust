//! Checkpoint 1: `splitpot`, a shared-expense ledger.
//!
//! Read `SPEC.md` first. It is the contract; this file only holds the public
//! names that the tests call, so that the tests compile. Everything else is
//! your design: the fields of `Ledger`, every helper function, and how the
//! parser walks through a line.
//!
//! Rules of the checkpoint (see `../README.md`): closed notes, one sitting,
//! 2 to 4 hours. Do not change the files in `tests/`. Do not change the
//! public signatures in this file. You may add private items freely.

// Starter bodies are `todo!()`, so parameters and fields are unused for now.
// Delete this line when you have implemented everything: the compiler's
// warnings are part of your feedback.
#![allow(dead_code, unused_variables)]

use std::error::Error;
use std::fmt;

// ---------------------------------------------------------------- amounts

/// Why a text is not an amount of money. SPEC section 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountError {
    Empty,
    Malformed,
    TooLarge,
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for AmountError {}

/// Reads an amount such as `12.50` and returns it in cents. SPEC section 2.
pub fn parse_amount(text: &str) -> Result<u64, AmountError> {
    todo!()
}

// ---------------------------------------------------------------- entries

/// One line of the ledger file: who paid how much for whom. SPEC section 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<'a> {
    pub payer: &'a str,
    pub cents: u64,
    pub sharers: Vec<&'a str>,
}

/// Why a line is not an entry. SPEC section 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryError {
    Syntax,
    Amount(AmountError),
    DuplicateSharer,
}

impl fmt::Display for EntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for EntryError {}

impl From<AmountError> for EntryError {
    fn from(error: AmountError) -> EntryError {
        todo!()
    }
}

/// Reads one entry line. SPEC section 3.
pub fn parse_entry(line: &str) -> Result<Entry<'_>, EntryError> {
    todo!()
}

// ----------------------------------------------------------------- ledger

/// A balance left the range of `i64`. SPEC section 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow;

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for Overflow {}

/// The balance of every person seen so far. SPEC section 4.
///
/// The fields are your design. Replace the placeholder.
#[derive(Debug)]
pub struct Ledger {
    _replace_me: (),
}

impl Ledger {
    /// A ledger without people.
    pub fn new() -> Ledger {
        todo!()
    }

    /// Applies one entry, or changes nothing and reports the overflow.
    pub fn add(&mut self, entry: &Entry<'_>) -> Result<(), Overflow> {
        todo!()
    }

    /// The balance of `name` in cents, or `None` for an unknown name.
    pub fn balance(&self, name: &str) -> Option<i64> {
        todo!()
    }

    /// Every name in the ledger, sorted.
    pub fn names(&self) -> Vec<&str> {
        todo!()
    }

    /// Gives up the ledger and returns `(name, balance)` pairs, sorted by name.
    pub fn into_balances(self) -> Vec<(String, i64)> {
        todo!()
    }
}

impl Default for Ledger {
    fn default() -> Ledger {
        Ledger::new()
    }
}

// ------------------------------------------------------------ whole files

/// What went wrong in a line of a ledger file. SPEC section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    Entry(EntryError),
    Overflow,
}

/// An error with the number of the line that caused it. SPEC section 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadError {
    pub line: usize,
    pub cause: Cause,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for ReadError {}

/// Reads a whole ledger file. SPEC section 5.
pub fn read_ledger(text: &str) -> Result<Ledger, ReadError> {
    todo!()
}

/// One line with the messages of `error` and of all its sources. SPEC
/// section 5.
pub fn report(error: &dyn Error) -> String {
    todo!()
}

// ---------------------------------------------------------------- settling

/// A payment that moves `cents` from one person to another. SPEC section 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transfer<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub cents: u64,
}

/// The balances do not add up to zero. SPEC section 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unbalanced;

impl fmt::Display for Unbalanced {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for Unbalanced {}

/// Plans the payments that bring every balance to zero. SPEC section 6.
pub fn settle(balances: &[(String, i64)]) -> Result<Vec<Transfer<'_>>, Unbalanced> {
    todo!()
}
