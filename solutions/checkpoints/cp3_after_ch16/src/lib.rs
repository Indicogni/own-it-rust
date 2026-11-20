//! Checkpoint 3: `matchday`, a league table. Reference solution.
//!
//! `COMMENTARY.md` next to this crate explains the decisions. Short version:
//! the teams live in a `BTreeMap`, so they are always in name order and a
//! `&str` finds a team without a new `String`; the order of the table is the
//! `Ord` of `Row`, written once and used by `sort`, by the heap in `top` and
//! by every caller; nothing is stored per match except a ring of five
//! outcomes per team.

use std::cmp::Ordering;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};
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
        match self {
            Outcome::Win => Outcome::Loss,
            Outcome::Draw => Outcome::Draw,
            Outcome::Loss => Outcome::Win,
        }
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
        match self.home.cmp(&self.away) {
            Ordering::Greater => Outcome::Win,
            Ordering::Equal => Outcome::Draw,
            Ordering::Less => Outcome::Loss,
        }
    }

    /// The same match seen by the away team.
    pub fn swapped(self) -> Score {
        Score {
            home: self.away,
            away: self.home,
        }
    }
}

impl From<(u8, u8)> for Score {
    fn from((home, away): (u8, u8)) -> Score {
        Score { home, away }
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
    /// Three `u32` values can add up to more than a `u32` holds, so the
    /// result is a `u64`.
    pub fn played(&self) -> u64 {
        u64::from(self.won) + u64::from(self.drawn) + u64::from(self.lost)
    }

    /// The difference of two `u32` values always fits in an `i64`.
    pub fn goal_difference(&self) -> i64 {
        i64::from(self.goals_for) - i64::from(self.goals_against)
    }
}

impl From<Score> for Tally {
    fn from(score: Score) -> Tally {
        let outcome = score.outcome();
        Tally {
            won: u32::from(outcome == Outcome::Win),
            drawn: u32::from(outcome == Outcome::Draw),
            lost: u32::from(outcome == Outcome::Loss),
            goals_for: u32::from(score.home),
            goals_against: u32::from(score.away),
        }
    }
}

impl Add for Tally {
    type Output = Tally;

    fn add(self, other: Tally) -> Tally {
        // The SPEC chooses saturation. Plain `+` would panic in a debug
        // build and wrap around in a release build.
        Tally {
            won: self.won.saturating_add(other.won),
            drawn: self.drawn.saturating_add(other.drawn),
            lost: self.lost.saturating_add(other.lost),
            goals_for: self.goals_for.saturating_add(other.goals_for),
            goals_against: self.goals_against.saturating_add(other.goals_against),
        }
    }
}

impl AddAssign for Tally {
    fn add_assign(&mut self, other: Tally) {
        // One definition of addition: `+=` is written with `+`.
        *self = *self + other;
    }
}

impl Sum for Tally {
    fn sum<I: Iterator<Item = Tally>>(tallies: I) -> Tally {
        tallies.fold(Tally::default(), Add::add)
    }
}

impl<'a> Sum<&'a Tally> for Tally {
    fn sum<I: Iterator<Item = &'a Tally>>(tallies: I) -> Tally {
        tallies.copied().sum()
    }
}

// ------------------------------------------------------------------- rows

/// One line of the table. SPEC section 4.
///
/// `PartialEq` is derived, so two rows are equal when all fields are equal.
/// `Ord` is written by hand and must agree with that; see `cmp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row<'a> {
    pub team: &'a str,
    pub tally: Tally,
    pub points: u64,
}

impl PartialOrd for Row<'_> {
    fn partial_cmp(&self, other: &Row<'_>) -> Option<Ordering> {
        // Always delegate, so that the two orders cannot drift apart.
        Some(self.cmp(other))
    }
}

impl Ord for Row<'_> {
    /// "Smaller" means "higher in the table". For the keys where more is
    /// better, `other` is on the left of the comparison.
    fn cmp(&self, other: &Row<'_>) -> Ordering {
        let (mine, theirs) = (&self.tally, &other.tally);
        other
            .points
            .cmp(&self.points)
            .then_with(|| theirs.goal_difference().cmp(&mine.goal_difference()))
            .then_with(|| theirs.goals_for.cmp(&mine.goals_for))
            .then_with(|| self.team.cmp(other.team))
            // The table is decided here. The last key only keeps `Ord` in
            // step with `==`: rows that differ in any field never compare
            // as `Equal`. (Equal goals for and equal difference imply equal
            // goals against, so three more fields are enough.)
            .then_with(|| {
                (theirs.won, theirs.drawn, theirs.lost).cmp(&(mine.won, mine.drawn, mine.lost))
            })
    }
}

// ----------------------------------------------------------------- errors

/// A name was given twice to `League::new`. SPEC section 7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateTeam(pub String);

impl fmt::Display for DuplicateTeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "team listed twice: {}", self.0)
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
        match self {
            RecordError::UnknownTeam(name) => write!(f, "unknown team: {name}"),
            RecordError::SameTeam => f.write_str("a team cannot play against itself"),
        }
    }
}

impl Error for RecordError {}

// ----------------------------------------------------------------- league

/// What the league knows about one team.
#[derive(Debug)]
struct Club {
    tally: Tally,
    /// Invariant: at most `League::FORM_LEN` outcomes, oldest at the front.
    /// A `VecDeque` because the oldest outcome leaves at the front and the
    /// newest enters at the back, both in constant time.
    form: VecDeque<Outcome>,
}

impl Club {
    fn new() -> Club {
        // The ring gets its full size now, so `record` never allocates.
        Club {
            tally: Tally::default(),
            form: VecDeque::with_capacity(League::FORM_LEN),
        }
    }

    /// Adds one match, with `score` already seen from this club's side.
    fn add(&mut self, score: Score) {
        self.tally += Tally::from(score);
        if self.form.len() == League::FORM_LEN {
            self.form.pop_front();
        }
        self.form.push_back(score.outcome());
    }
}

type Rule = Box<dyn Fn(&Tally) -> u64>;
type Observer = Box<dyn FnMut(&str, &str, Score)>;

/// The teams of a league and their records. SPEC section 5.
pub struct League {
    /// Keyed by name. A `BTreeMap` keeps the names in the order of `str`
    /// and looks a `&str` up without building a `String`.
    clubs: BTreeMap<String, Club>,
    rule: Rule,
    /// A `Vec` because the observers are called in the order in which they
    /// were registered, and never searched or removed.
    observers: Vec<Observer>,
}

impl League {
    /// How many outcomes the form of a team holds at most.
    pub const FORM_LEN: usize = 5;

    pub fn new<I>(teams: I) -> Result<League, DuplicateTeam>
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        let mut clubs = BTreeMap::new();
        for name in teams {
            // `into` moves a `String` and copies a `&str`: one buffer per
            // name either way. The entry tells us whether the name is new
            // with a single search of the map.
            match clubs.entry(name.into()) {
                Entry::Vacant(place) => {
                    place.insert(Club::new());
                }
                Entry::Occupied(taken) => return Err(DuplicateTeam(taken.key().clone())),
            }
        }
        Ok(League {
            clubs,
            rule: Box::new(|tally| 3 * u64::from(tally.won) + u64::from(tally.drawn)),
            observers: Vec::new(),
        })
    }

    /// Replaces the rule that turns a tally into points.
    pub fn with_rule(mut self, rule: impl Fn(&Tally) -> u64 + 'static) -> League {
        self.rule = Box::new(rule);
        self
    }

    /// Registers a callback for every match that is recorded from now on.
    pub fn on_record(&mut self, observer: impl FnMut(&str, &str, Score) + 'static) {
        self.observers.push(Box::new(observer));
    }

    /// Reports one match.
    pub fn record(
        &mut self,
        home: &str,
        away: &str,
        score: impl Into<Score>,
    ) -> Result<(), RecordError> {
        // Phase 1: every check. Nothing has changed when one of them fails.
        for team in [home, away] {
            if !self.clubs.contains_key(team) {
                return Err(RecordError::UnknownTeam(team.to_string()));
            }
        }
        if home == away {
            return Err(RecordError::SameTeam);
        }

        // Phase 2: the changes. Each team sees the score from its own side.
        let score = score.into();
        for (team, seen) in [(home, score), (away, score.swapped())] {
            if let Some(club) = self.clubs.get_mut(team) {
                club.add(seen);
            }
        }
        for observer in &mut self.observers {
            observer(home, away, score);
        }
        Ok(())
    }

    /// Every name, in the order of `str`.
    pub fn teams(&self) -> impl Iterator<Item = &str> {
        self.clubs.keys().map(String::as_str)
    }

    pub fn tally(&self, team: &str) -> Option<&Tally> {
        self.clubs.get(team).map(|club| &club.tally)
    }

    /// The most recent outcomes of a team, oldest first.
    pub fn form(
        &self,
        team: &str,
    ) -> Option<impl DoubleEndedIterator<Item = Outcome> + ExactSizeIterator> {
        // The iterator of a `VecDeque` already has both abilities, and
        // `copied` keeps them.
        self.clubs.get(team).map(|club| club.form.iter().copied())
    }

    /// One row per team, in name order. The rule runs once for each row,
    /// when the row is made, and never inside a comparison.
    fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.clubs.iter().map(|(name, club)| Row {
            team: name,
            tally: club.tally,
            points: (self.rule)(&club.tally),
        })
    }

    /// The whole table, leader first.
    pub fn standings(&self) -> Vec<Row<'_>> {
        let mut rows: Vec<Row<'_>> = self.rows().collect();
        // No two rows are equal (the names differ), so the faster unstable
        // sort gives the same result as the stable one.
        rows.sort_unstable();
        rows
    }

    /// The first `k` rows of the table.
    pub fn top(&self, k: usize) -> Vec<Row<'_>> {
        // `k` may be `usize::MAX`, so `k + 1` is computed only after `k`
        // has been limited to the number of teams.
        let k = k.min(self.clubs.len());
        if k == 0 {
            return Vec::new();
        }
        // A max-heap of rows has the *lowest-ranked* row on top, because
        // "greater" means "lower in the table". Pushing a row and removing
        // the top whenever there are more than `k` keeps the best `k`.
        let mut best = BinaryHeap::with_capacity(k + 1);
        for row in self.rows() {
            best.push(row);
            if best.len() > k {
                best.pop();
            }
        }
        // Ascending order is table order, and the sort reuses the buffer.
        best.into_sorted_vec()
    }

    /// The pairings of a full season.
    pub fn fixtures(&self) -> Fixtures<'_> {
        // One allocation: a list of the names, so that a pairing can be
        // found by position. The pairings themselves are never stored.
        let teams: Vec<&str> = self.teams().collect();
        let end = teams.len() * teams.len().saturating_sub(1);
        Fixtures {
            teams,
            next: 0,
            end,
        }
    }
}

impl Index<&str> for League {
    type Output = Tally;

    /// Panics for an unknown team, as indexing a map does.
    fn index(&self, team: &str) -> &Tally {
        match self.tally(team) {
            Some(tally) => tally,
            None => panic!("no team named {team:?} in the league"),
        }
    }
}

// --------------------------------------------------------------- fixtures

/// An iterator over the pairings `(home, away)` of a full season. SPEC
/// section 6.
///
/// The pairings are numbered from 0 to `n * (n - 1) - 1` in the order in
/// which they are yielded. The iterator holds only the range of numbers that
/// are still to come, and computes a pairing from its number.
pub struct Fixtures<'a> {
    /// The names, sorted. A `Vec` because a pairing is found by position.
    teams: Vec<&'a str>,
    /// Invariant: `next <= end <= n * (n - 1)`.
    next: usize,
    end: usize,
}

impl<'a> Fixtures<'a> {
    /// The pairing with this number. Only called with `number < end`, which
    /// implies at least two teams.
    fn pairing(&self, number: usize) -> (&'a str, &'a str) {
        // Each team is at home in `n - 1` pairings in a row. Its guests are
        // all teams in order, with the home team itself left out.
        let guests = self.teams.len() - 1;
        let home = number / guests;
        let slot = number % guests;
        let away = if slot < home { slot } else { slot + 1 };
        (self.teams[home], self.teams[away])
    }
}

impl<'a> Iterator for Fixtures<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        if self.next == self.end {
            return None;
        }
        let pairing = self.pairing(self.next);
        self.next += 1;
        Some(pairing)
    }

    /// `ExactSizeIterator::len` relies on this being exact.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.end - self.next;
        (left, Some(left))
    }
}

impl DoubleEndedIterator for Fixtures<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.next == self.end {
            return None;
        }
        self.end -= 1;
        Some(self.pairing(self.end))
    }
}

impl ExactSizeIterator for Fixtures<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_numbers_skip_the_home_team() {
        let league = League::new(["a", "b", "c"]).unwrap();
        let fixtures = league.fixtures();
        assert_eq!(fixtures.pairing(0), ("a", "b"));
        assert_eq!(fixtures.pairing(2), ("b", "a"));
        assert_eq!(fixtures.pairing(3), ("b", "c"));
        assert_eq!(fixtures.pairing(5), ("c", "b"));
    }

    #[test]
    fn form_ring_never_grows() {
        let mut club = Club::new();
        let capacity = club.form.capacity();
        for goals in 0..20 {
            club.add(Score {
                home: goals,
                away: 3,
            });
        }
        assert_eq!(club.form.len(), League::FORM_LEN);
        assert_eq!(club.form.capacity(), capacity);
        assert_eq!(club.tally.played(), 20);
    }

    #[test]
    fn a_club_sees_the_score_from_its_own_side() {
        let mut home = Club::new();
        let mut away = Club::new();
        let score = Score { home: 3, away: 1 };
        home.add(score);
        away.add(score.swapped());
        assert_eq!(
            home.tally + away.tally,
            Tally {
                won: 1,
                drawn: 0,
                lost: 1,
                goals_for: 4,
                goals_against: 4
            }
        );
        assert_eq!(home.form, [Outcome::Win]);
        assert_eq!(away.form, [Outcome::Loss]);
    }

    #[test]
    fn rows_come_in_name_order_before_sorting() {
        let mut league = League::new(["b", "c", "a"]).unwrap();
        league.record("c", "a", (1, 0)).unwrap();
        let rows: Vec<(&str, u64)> = league.rows().map(|row| (row.team, row.points)).collect();
        assert_eq!(rows, [("a", 0), ("b", 0), ("c", 3)]);
    }

    #[test]
    fn the_heap_top_is_the_lowest_row() {
        let low = Row {
            team: "low",
            tally: Tally::default(),
            points: 1,
        };
        let high = Row {
            team: "high",
            tally: Tally::default(),
            points: 9,
        };
        let mut heap = BinaryHeap::from([high, low]);
        assert_eq!(heap.pop(), Some(low));
    }
}
