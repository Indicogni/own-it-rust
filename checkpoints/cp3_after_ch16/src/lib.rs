//! Checkpoint 3: `matchday`, a league table.
//!
//! Read `SPEC.md` first. It is the contract; this file only holds the public
//! names that the tests call, so that the tests compile. Everything else is
//! your design: the fields of `League` and `Fixtures`, the collections that
//! hold the data, private types and helper functions.
//!
//! Rules of the checkpoint (see `../README.md`): closed notes, one sitting,
//! 2 to 4 hours. Do not change the files in `tests/`. Do not change a public
//! name or signature. A struct marked "your design" has a placeholder field;
//! replace it. Trait implementations whose body is `todo!()` may be replaced
//! by a derive.

// Starter bodies are `todo!()`, so parameters and fields are unused for now.
// Delete this line when you have implemented everything: the compiler's
// warnings are part of your feedback.
#![allow(dead_code, unused_variables, unreachable_code)]

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Index, Not};

// ---------------------------------------------------------------- results

/// The result of a match for one of the two teams. SPEC section 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Win,
    Draw,
    Loss,
}

impl Not for Outcome {
    type Output = Outcome;

    fn not(self) -> Outcome {
        todo!()
    }
}

/// The goals of the home team and of the away team. SPEC section 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Score {
    pub home: u8,
    pub away: u8,
}

impl Score {
    /// The result for the home team.
    pub fn outcome(self) -> Outcome {
        todo!()
    }

    /// The same match seen by the away team.
    pub fn swapped(self) -> Score {
        todo!()
    }
}

impl From<(u8, u8)> for Score {
    fn from(goals: (u8, u8)) -> Score {
        todo!()
    }
}

// ---------------------------------------------------------------- tallies

/// The record of one team. SPEC section 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tally {
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
    pub goals_for: u32,
    pub goals_against: u32,
}

impl Tally {
    pub fn played(&self) -> u64 {
        todo!()
    }

    pub fn goal_difference(&self) -> i64 {
        todo!()
    }
}

impl From<Score> for Tally {
    fn from(score: Score) -> Tally {
        todo!()
    }
}

impl Add for Tally {
    type Output = Tally;

    fn add(self, other: Tally) -> Tally {
        todo!()
    }
}

impl AddAssign for Tally {
    fn add_assign(&mut self, other: Tally) {
        todo!()
    }
}

impl Sum for Tally {
    fn sum<I: Iterator<Item = Tally>>(tallies: I) -> Tally {
        todo!()
    }
}

impl<'a> Sum<&'a Tally> for Tally {
    fn sum<I: Iterator<Item = &'a Tally>>(tallies: I) -> Tally {
        todo!()
    }
}

// ------------------------------------------------------------------- rows

/// One line of the table. SPEC section 4.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    pub team: &'a str,
    pub tally: Tally,
    pub points: u64,
}

impl PartialEq for Row<'_> {
    fn eq(&self, other: &Row<'_>) -> bool {
        todo!()
    }
}

impl Eq for Row<'_> {}

impl PartialOrd for Row<'_> {
    fn partial_cmp(&self, other: &Row<'_>) -> Option<Ordering> {
        todo!()
    }
}

impl Ord for Row<'_> {
    fn cmp(&self, other: &Row<'_>) -> Ordering {
        todo!()
    }
}

// ----------------------------------------------------------------- errors

/// A name was given twice to `League::new`. SPEC section 7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateTeam(pub String);

impl fmt::Display for DuplicateTeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for DuplicateTeam {}

/// Why a match was not recorded. SPEC section 7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordError {
    UnknownTeam(String),
    SameTeam,
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for RecordError {}

// ----------------------------------------------------------------- league

/// The teams of a league and their records. SPEC section 5.
///
/// The fields are your design. Replace the placeholder.
pub struct League {
    _replace_me: (),
}

impl League {
    /// How many outcomes the form of a team holds at most.
    pub const FORM_LEN: usize = 5;

    pub fn new<I>(teams: I) -> Result<League, DuplicateTeam>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        todo!()
    }

    /// Replaces the rule that turns a tally into points.
    pub fn with_rule(self, rule: impl Fn(&Tally) -> u64 + 'static) -> League {
        todo!()
    }

    /// Registers a callback for every match that is recorded from now on.
    pub fn on_record(&mut self, observer: impl FnMut(&str, &str, Score) + 'static) {
        todo!()
    }

    /// Reports one match.
    pub fn record(
        &mut self,
        home: &str,
        away: &str,
        score: impl Into<Score>,
    ) -> Result<(), RecordError> {
        todo!()
    }

    /// Every name, in the order of `str`.
    pub fn teams(&self) -> impl Iterator<Item = &str> {
        // Replace both lines. The second one only gives the unfinished
        // function an iterator type, so that the file compiles.
        todo!();
        std::iter::empty()
    }

    pub fn tally(&self, team: &str) -> Option<&Tally> {
        todo!()
    }

    /// The most recent outcomes of a team, oldest first.
    pub fn form(
        &self,
        team: &str,
    ) -> Option<impl DoubleEndedIterator<Item = Outcome> + ExactSizeIterator> {
        // Replace both lines, as in `teams`.
        todo!();
        None::<std::iter::Empty<Outcome>>
    }

    /// The whole table, leader first.
    pub fn standings(&self) -> Vec<Row<'_>> {
        todo!()
    }

    /// The first `k` rows of the table.
    pub fn top(&self, k: usize) -> Vec<Row<'_>> {
        todo!()
    }

    /// The pairings of a full season.
    pub fn fixtures(&self) -> Fixtures<'_> {
        todo!()
    }
}

impl Index<&str> for League {
    type Output = Tally;

    fn index(&self, team: &str) -> &Tally {
        todo!()
    }
}

// --------------------------------------------------------------- fixtures

/// An iterator over the pairings `(home, away)` of a full season. SPEC
/// section 6.
///
/// The fields are your design. Replace the placeholder.
pub struct Fixtures<'a> {
    _replace_me: &'a (),
}

impl<'a> Iterator for Fixtures<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        todo!()
    }
}

impl DoubleEndedIterator for Fixtures<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl ExactSizeIterator for Fixtures<'_> {}
