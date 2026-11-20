//! Checkpoint 1: `splitpot`, a shared-expense ledger. Reference solution.
//!
//! `COMMENTARY.md` next to this crate explains the decisions. Short version:
//! money is whole cents in integers, text is borrowed and never copied until
//! a name enters the ledger, and every change to the ledger is computed
//! completely before the first balance is written.

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
        f.write_str(match self {
            AmountError::Empty => "amount is empty",
            AmountError::Malformed => "amount is not a number with at most two decimal places",
            AmountError::TooLarge => "amount is too large",
        })
    }
}

impl Error for AmountError {}

/// The value of a text that consists of ASCII digits only, or `None` when
/// the text is empty or contains anything else.
///
/// `str::parse::<u64>` is not used for the check: it accepts a leading `+`,
/// which the SPEC forbids. Checking the bytes first also makes the meaning
/// of a later parse error certain: it can only be an overflow.
fn all_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// Reads an amount such as `12.50` and returns it in cents. SPEC section 2.
pub fn parse_amount(text: &str) -> Result<u64, AmountError> {
    if text.is_empty() {
        return Err(AmountError::Empty);
    }
    let (units, fraction) = match text.split_once('.') {
        Some((units, fraction)) => (units, Some(fraction)),
        None => (text, None),
    };
    if !all_digits(units) {
        return Err(AmountError::Malformed);
    }
    // "5.2" means 20 cents, "5.02" means 2 cents.
    let cents = match fraction {
        None => 0,
        Some(digits) if all_digits(digits) => match *digits.as_bytes() {
            [tens] => u64::from(tens - b'0') * 10,
            [tens, ones] => u64::from(tens - b'0') * 10 + u64::from(ones - b'0'),
            _ => return Err(AmountError::Malformed),
        },
        Some(_) => return Err(AmountError::Malformed),
    };
    // Only digits are left, so the one possible parse error is an overflow.
    let units: u64 = units.parse().map_err(|_| AmountError::TooLarge)?;
    units
        .checked_mul(100)
        .and_then(|whole| whole.checked_add(cents))
        .ok_or(AmountError::TooLarge)
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
        f.write_str(match self {
            EntryError::Syntax => "expected: <payer> paid <amount> for <name>, <name>, ...",
            // The detail is in `source()`. Printing it here as well would
            // show it twice in a report that walks the chain.
            EntryError::Amount(_) => "invalid amount",
            EntryError::DuplicateSharer => "a name appears twice after `for`",
        })
    }
}

impl Error for EntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            EntryError::Amount(cause) => Some(cause),
            EntryError::Syntax | EntryError::DuplicateSharer => None,
        }
    }
}

impl From<AmountError> for EntryError {
    fn from(error: AmountError) -> EntryError {
        EntryError::Amount(error)
    }
}

/// Cuts the first word off `rest` and returns it. Afterwards `rest` starts
/// at the first character after the white space that followed the word.
///
/// The word has the lifetime of the line (`'a`), not of the short borrow of
/// the variable `rest`. That is why the two lifetimes are written out.
fn next_word<'a>(rest: &mut &'a str) -> Option<&'a str> {
    let text = rest.trim_start();
    if text.is_empty() {
        return None;
    }
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    let (word, tail) = text.split_at(end);
    *rest = tail.trim_start();
    Some(word)
}

fn is_name(text: &str) -> bool {
    !text.is_empty() && !text.contains(|c: char| c == ',' || c.is_whitespace())
}

/// Reads one entry line. SPEC section 3.
pub fn parse_entry(line: &str) -> Result<Entry<'_>, EntryError> {
    let mut rest = line;
    let payer = next_word(&mut rest)
        .filter(|word| is_name(word))
        .ok_or(EntryError::Syntax)?;
    if next_word(&mut rest) != Some("paid") {
        return Err(EntryError::Syntax);
    }
    let amount = next_word(&mut rest).ok_or(EntryError::Syntax)?;
    let cents = parse_amount(amount)?;
    if next_word(&mut rest) != Some("for") {
        return Err(EntryError::Syntax);
    }

    // The only allocation of this function: the list itself. The names in
    // it are slices of `line`.
    let mut sharers: Vec<&str> = Vec::new();
    for item in rest.split(',') {
        let name = item.trim();
        if !is_name(name) {
            // Also covers an empty list, "a,,b" and a trailing comma: each
            // of them produces an empty item.
            return Err(EntryError::Syntax);
        }
        if sharers.contains(&name) {
            return Err(EntryError::DuplicateSharer);
        }
        sharers.push(name);
    }
    Ok(Entry {
        payer,
        cents,
        sharers,
    })
}

// ----------------------------------------------------------------- ledger

/// A balance left the range of `i64`. SPEC section 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow;

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("balance out of range")
    }
}

impl Error for Overflow {}

/// The balance of every person seen so far. SPEC section 4.
///
/// Invariant: `accounts` is sorted by name and has no name twice. This makes
/// lookups a binary search and `into_balances` a plain move.
#[derive(Debug, Default)]
pub struct Ledger {
    accounts: Vec<(String, i64)>,
}

impl Ledger {
    /// A ledger without people.
    pub fn new() -> Ledger {
        Ledger::default()
    }

    /// `Ok(index)` of the account of `name`, or `Err(index)` where it would
    /// have to be inserted to keep the order.
    fn position(&self, name: &str) -> Result<usize, usize> {
        self.accounts
            .binary_search_by(|(known, _)| known.as_str().cmp(name))
    }

    /// Applies one entry, or changes nothing and reports the overflow.
    pub fn add(&mut self, entry: &Entry<'_>) -> Result<(), Overflow> {
        // The SPEC: an entry without sharers changes nothing. It also keeps
        // the division below away from zero.
        if entry.sharers.is_empty() {
            return Ok(());
        }
        // A `usize` always fits in a `u64` on the platforms Rust supports.
        let count = entry.sharers.len() as u64;
        let share = entry.cents / count;
        let extra = entry.cents % count;

        // Phase 1: compute every new balance. Nothing is written yet, so an
        // overflow can simply return. The change of one person is computed
        // in i128 because the payer's credit and the payer's own share may
        // cancel out even when the credit alone does not fit in an i64.
        let mut changes: Vec<(&str, i64)> = Vec::with_capacity(entry.sharers.len() + 1);
        let mut payer_is_sharer = false;
        for (index, &name) in entry.sharers.iter().enumerate() {
            let owed = share + u64::from((index as u64) < extra);
            let mut change = -i128::from(owed);
            if name == entry.payer {
                payer_is_sharer = true;
                change += i128::from(entry.cents);
            }
            changes.push((name, self.checked_balance(name, change)?));
        }
        if !payer_is_sharer {
            let change = i128::from(entry.cents);
            changes.push((entry.payer, self.checked_balance(entry.payer, change)?));
        }

        // Phase 2: write. Nothing in here can fail.
        for (name, balance) in changes {
            match self.position(name) {
                Ok(index) => self.accounts[index].1 = balance,
                Err(index) => self.accounts.insert(index, (name.to_string(), balance)),
            }
        }
        Ok(())
    }

    /// The balance of `name` after `change`, if it fits in an `i64`.
    fn checked_balance(&self, name: &str, change: i128) -> Result<i64, Overflow> {
        let old = self.balance(name).unwrap_or(0);
        i64::try_from(i128::from(old) + change).map_err(|_| Overflow)
    }

    /// The balance of `name` in cents, or `None` for an unknown name.
    pub fn balance(&self, name: &str) -> Option<i64> {
        self.position(name).ok().map(|index| self.accounts[index].1)
    }

    /// Every name in the ledger, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.accounts
            .iter()
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Gives up the ledger and returns `(name, balance)` pairs, sorted by name.
    pub fn into_balances(self) -> Vec<(String, i64)> {
        // Already in the right shape and order: a move, no copy.
        self.accounts
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
        let what = match self.cause {
            Cause::Entry(_) => "invalid entry",
            Cause::Overflow => "balance out of range",
        };
        write!(f, "line {}: {what}", self.line)
    }
}

impl Error for ReadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.cause {
            Cause::Entry(cause) => Some(cause),
            Cause::Overflow => None,
        }
    }
}

/// Reads a whole ledger file. SPEC section 5.
pub fn read_ledger(text: &str) -> Result<Ledger, ReadError> {
    let mut ledger = Ledger::new();
    for (index, line) in text.lines().enumerate() {
        let content = line.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        // The closure adds the line number to whichever cause occurred.
        let fail = |cause| ReadError {
            line: index + 1,
            cause,
        };
        let entry = parse_entry(content).map_err(|error| fail(Cause::Entry(error)))?;
        ledger
            .add(&entry)
            .map_err(|Overflow| fail(Cause::Overflow))?;
    }
    Ok(ledger)
}

/// One line with the messages of `error` and of all its sources. SPEC
/// section 5.
pub fn report(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut next = error.source();
    while let Some(cause) = next {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        next = cause.source();
    }
    text
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
        f.write_str("balances do not add up to zero")
    }
}

impl Error for Unbalanced {}

/// Plans the payments that bring every balance to zero. SPEC section 6.
pub fn settle(balances: &[(String, i64)]) -> Result<Vec<Transfer<'_>>, Unbalanced> {
    // The sum of many i64 values needs a wider type. An i64 sum could wrap
    // around to exactly zero and hide that the input is unbalanced.
    let sum: i128 = balances.iter().map(|&(_, cents)| i128::from(cents)).sum();
    if sum != 0 {
        return Err(Unbalanced);
    }

    // A working copy of the amounts. The names stay where they are.
    let mut open: Vec<i64> = balances.iter().map(|&(_, cents)| cents).collect();
    let mut transfers = Vec::new();
    loop {
        // Strict comparisons keep the earliest index among equal balances.
        let mut debtor = 0;
        let mut creditor = 0;
        for (index, &cents) in open.iter().enumerate() {
            if cents < open[debtor] {
                debtor = index;
            }
            if cents > open[creditor] {
                creditor = index;
            }
        }
        // The sum is zero, so a debt exists exactly when a credit exists.
        if open.is_empty() || open[debtor] >= 0 {
            break;
        }

        // `-open[debtor]` would overflow for i64::MIN; `unsigned_abs` cannot.
        let debt = open[debtor].unsigned_abs();
        let credit = open[creditor].unsigned_abs();
        let cents = debt.min(credit);
        // `cents` is at most the credit, which is at most i64::MAX, so both
        // updates stay in range.
        open[debtor] = open[debtor].wrapping_add_unsigned(cents);
        open[creditor] = open[creditor].wrapping_sub_unsigned(cents);
        transfers.push(Transfer {
            from: &balances[debtor].0,
            to: &balances[creditor].0,
            cents,
        });
    }
    Ok(transfers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_word_moves_the_rest_forward() {
        let mut rest = "  ana   paid 3";
        assert_eq!(next_word(&mut rest), Some("ana"));
        assert_eq!(rest, "paid 3");
        assert_eq!(next_word(&mut rest), Some("paid"));
        assert_eq!(next_word(&mut rest), Some("3"));
        assert_eq!(next_word(&mut rest), None);
        assert_eq!(rest, "");
    }

    #[test]
    fn the_remainder_goes_to_the_first_sharers() {
        let mut ledger = Ledger::new();
        let entry = parse_entry("x paid 0.05 for a, b, c").unwrap();
        ledger.add(&entry).unwrap();
        assert_eq!(ledger.balance("a"), Some(-2));
        assert_eq!(ledger.balance("b"), Some(-2));
        assert_eq!(ledger.balance("c"), Some(-1));
        assert_eq!(ledger.balance("x"), Some(5));
    }

    #[test]
    fn a_name_has_no_white_space_and_no_comma() {
        assert!(is_name("Zoë"));
        assert!(is_name("x#1"));
        assert!(!is_name(""));
        assert!(!is_name("a b"));
        assert!(!is_name("a,b"));
        assert!(
            !is_name("a\u{a0}b"),
            "U+00A0 is white space for `char::is_whitespace`"
        );
    }

    #[test]
    fn only_ascii_digits_are_digits() {
        assert!(all_digits("0123456789"));
        assert!(!all_digits(""));
        assert!(!all_digits("+5"));
        assert!(!all_digits("٣"));
    }

    #[test]
    fn a_checked_balance_reports_both_ends_of_the_range() {
        let ledger = read_ledger("ana paid 92233720368547758.07 for ben").unwrap();
        assert_eq!(ledger.checked_balance("ana", 1), Err(Overflow));
        assert_eq!(ledger.checked_balance("ben", -1), Ok(i64::MIN));
        assert_eq!(ledger.checked_balance("ben", -2), Err(Overflow));
        assert_eq!(ledger.checked_balance("new", 7), Ok(7));
    }

    #[test]
    fn accounts_stay_sorted() {
        let ledger = read_ledger("zed paid 1 for mo\nabe paid 1 for zed").unwrap();
        assert_eq!(ledger.names(), ["abe", "mo", "zed"]);
    }
}
