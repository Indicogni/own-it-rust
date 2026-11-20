# Checkpoint 3 specification: `matchday`

[All checkpoints](../README.md) · [Rubric](RUBRIC.md)

`matchday` is a library that keeps the table of a sports league. Somebody
reports the result of each match. The library keeps one record per team,
ranks the teams, shows the recent form of a team, and lists the pairings of
a season.

```text
Kestrels 2:1 Otters      Herons 1:1 Kestrels      Otters 2:1 Herons

            W  D  L  GF  GA  points
Kestrels    1  1  0   3   2       4
Otters      1  0  1   3   3       3
Herons      0  1  1   2   3       1
```

This document is the contract. Where it says "must", a test checks it. The
public names and signatures are already in `src/lib.rs`, so that the tests
compile. Everything behind them is yours to design: the fields of `League`
and `Fixtures`, which collections hold the data, private types and helper
functions.

## 1. Rules that apply everywhere

1. **Team names** are compared exactly, byte by byte. `Otters` and `otters`
   are two teams. Every string is a name, also the empty string.
2. **No function may panic** for any input, in a debug or in a release
   build. The one exception is indexing a league with an unknown name
   (section 5.4). No function may print.
3. **No floating-point type** appears anywhere.
4. Only the standard library may be used. `unsafe` is not allowed.

## 2. `Outcome` and `Score`

```text
pub enum Outcome { Win, Draw, Loss }
pub struct Score { pub home: u8, pub away: u8 }
impl Score { pub fn outcome(self) -> Outcome;  pub fn swapped(self) -> Score }
```

An `Outcome` is the result of a match for one of the two teams.

- `!outcome` is the same match seen by the other team: `!Win` is `Loss`,
  `!Loss` is `Win`, and `!Draw` is `Draw`.

A `Score` holds the goals of the home team and of the away team.

- `Score::from((2, 1))` is `Score { home: 2, away: 1 }`.
- `outcome()` is the result for the home team: `Win` when `home` is
  greater than `away`, `Loss` when it is smaller, `Draw` when both are
  equal.
- `swapped()` is the same match seen by the away team: the two numbers
  change places.

## 3. `Tally`

```text
pub struct Tally { pub won: u32, pub drawn: u32, pub lost: u32,
                   pub goals_for: u32, pub goals_against: u32 }
impl Tally { pub fn played(&self) -> u64;  pub fn goal_difference(&self) -> i64 }
```

A `Tally` is the record of one team: matches won, drawn and lost, goals
scored and goals conceded.

- `Tally::default()` has zero in every field.
- `Tally::from(score)` is the tally of the **home** team after that one
  match. `Tally::from(Score { home: 2, away: 1 })` has `won: 1`,
  `goals_for: 2`, `goals_against: 1`, and zero in the other fields.
- `a + b` adds field by field. A field that would pass `u32::MAX` stays at
  `u32::MAX`. `a += b` does the same to `a`.
- An iterator of `Tally` values, and an iterator of `&Tally` references,
  can be added up with `.sum::<Tally>()`. The sum of nothing is
  `Tally::default()`.
- `played()` is `won + drawn + lost`. `goal_difference()` is
  `goals_for - goals_against`. Both are exact for every possible tally.

## 4. `Row` and the order of the table

```text
pub struct Row<'a> { pub team: &'a str, pub tally: Tally, pub points: u64 }
```

A `Row` is one line of the table. Rows have a total order, and "smaller"
means "higher in the table": if `a < b`, row `a` stands above row `b`.
Compare two rows by these rules in turn. The first rule that finds a
difference decides.

1. The row with more `points` is smaller.
2. The row with the greater goal difference is smaller.
3. The row with more `goals_for` is smaller.
4. The row whose `team` is smaller in the order of `str` is smaller.
5. If the rows are still tied, the order is your decision, with one
   condition: `a.cmp(&b)` is `Ordering::Equal` **only** when `a == b`.

`a == b` holds exactly when all three fields are equal. `Row` implements
`PartialEq`, `Eq`, `PartialOrd` and `Ord`, and the four agree with each
other: `a.partial_cmp(&b)` is always `Some(a.cmp(&b))`.

## 5. `League`

```text
impl League {
    pub const FORM_LEN: usize = 5;
    pub fn new<I>(teams: I) -> Result<League, DuplicateTeam>
        where I: IntoIterator, I::Item: Into<String>;
    pub fn with_rule(self, rule: impl Fn(&Tally) -> u64 + 'static) -> League;
    pub fn on_record(&mut self, observer: impl FnMut(&str, &str, Score) + 'static);
    pub fn record(&mut self, home: &str, away: &str, score: impl Into<Score>)
        -> Result<(), RecordError>;
    pub fn teams(&self) -> impl Iterator<Item = &str>;
    pub fn tally(&self, team: &str) -> Option<&Tally>;
    pub fn form(&self, team: &str)
        -> Option<impl DoubleEndedIterator<Item = Outcome> + ExactSizeIterator>;
    pub fn standings(&self) -> Vec<Row<'_>>;
    pub fn top(&self, k: usize) -> Vec<Row<'_>>;
    pub fn fixtures(&self) -> Fixtures<'_>;
}
impl Index<&str> for League { type Output = Tally; }
```

### 5.1 `new`

`new` takes the names of the teams. They may come as `&str` or as `String`,
from an array, a `Vec` or any other iterator. A league may have no teams.

- Every team starts with `Tally::default()` and no matches.
- If a name occurs twice, the result is `Err(DuplicateTeam(name))` with the
  first name that is seen for the second time.
- The league keeps its teams forever. No team is added or removed later.

### 5.2 Points: `with_rule`

A league turns a tally into points with a *rule*. The rule of a new league
gives 3 points for each match won and 1 point for each match drawn.
`with_rule` takes the league and returns it with another rule:

```text
let league = League::new(["Herons", "Otters"])?.with_rule(|t| 2 * u64::from(t.won));
```

The league calls the rule only inside `standings` and `top`, and there
**exactly once for each team** in each call.

### 5.3 `record` and `on_record`

`record(home, away, score)` reports one match. The score may be given as a
`Score` or as a pair `(home_goals, away_goals)`. The checks run in this
order, and the first one that fails decides.

1. `home` is not a team of the league:
   `Err(RecordError::UnknownTeam(home.to_string()))`.
2. `away` is not a team of the league:
   `Err(RecordError::UnknownTeam(away.to_string()))`.
3. `home` and `away` are the same team: `Err(RecordError::SameTeam)`.

If all checks pass:

- `Tally::from(score)` is added to the tally of the home team, and
  `Tally::from(score.swapped())` to the tally of the away team;
- the outcome is added to the form of both teams (section 5.5);
- then every observer is called once, with `(home, away, score)`.

*Observers* are callbacks that were registered with `on_record`. They are
called in the order in which they were registered. They stay registered
for the life of the league.

A `record` that returns `Err` changes nothing and calls no observer. The
same two teams may meet any number of times.

### 5.4 Reading

- `teams()` yields every name once, in the order of `str` (`"Zebras"`
  comes before `"ants"`).
- `tally(team)` is the tally of a team, or `None` for an unknown name.
- `league[team]` is the same tally. For an unknown name it panics, and the
  panic message contains the name.

### 5.5 `form`

The *form* of a team is the list of the outcomes of its most recent
matches, seen from that team, oldest first. It holds at most
`League::FORM_LEN` (5) outcomes: when a sixth match is recorded, the oldest
outcome leaves the form.

`form(team)` is `None` for an unknown name. Otherwise it is an iterator
over the form. It can be read from both ends and it knows its length. A
team without matches has an empty form.

### 5.6 `standings` and `top`

- `standings()` returns one `Row` for every team: its name, its tally, and
  its points according to the rule. The rows are sorted in the order of
  section 4, so the leader is at index 0.
- `top(k)` returns the first `k` rows of the standings, or all rows when
  the league has fewer than `k` teams. Every `k` from 0 to `usize::MAX` is
  valid.

The `team` of every row is borrowed from the league: it is the league's own
string, not a copy.

## 6. `Fixtures`

```text
pub struct Fixtures<'a> { /* your design */ }
impl<'a> Iterator for Fixtures<'a> { type Item = (&'a str, &'a str); }
impl DoubleEndedIterator for Fixtures<'_>
impl ExactSizeIterator for Fixtures<'_>
```

`league.fixtures()` lists the pairings of a full season, in which every
team is at home against every other team once. It is an iterator over
pairs `(home, away)`.

- It yields every ordered pair of two different teams exactly once. A
  league with `n` teams has `n × (n − 1)` pairings. A league with no teams
  or with one team has none.
- The pairs come in the order of `home`, and for the same `home` in the
  order of `away`, both in the order of `str`. For the teams `a`, `b`, `c`:
  `(a, b)`, `(a, c)`, `(b, a)`, `(b, c)`, `(c, a)`, `(c, b)`.
- `next_back()` yields the pairs from the other end. Calls of `next()` and
  `next_back()` may be mixed; together they yield every pair once.
- `len()` and `size_hint()` report exactly how many pairs are left.
- After the last pair, `next()` and `next_back()` return `None` every
  time.
- The names are borrowed from the league.

The iterator does not look at the recorded matches.

## 7. Errors

```text
pub struct DuplicateTeam(pub String);
pub enum RecordError { UnknownTeam(String), SameTeam }
```

Both types implement `std::error::Error` and have no source. `Display`
prints exactly:

| value | text |
|---|---|
| `DuplicateTeam("Otters".to_string())` | `team listed twice: Otters` |
| `RecordError::UnknownTeam("Owls".to_string())` | `unknown team: Owls` |
| `RecordError::SameTeam` | `a team cannot play against itself` |

## 8. Cost

1. **Reading is free.** `tally`, indexing, `teams` and `form`, and reading
   the iterators they return, must not allocate memory.
2. **The league does not grow with the number of matches.** A successful
   `record` in a league without observers must not allocate, except during
   the first five matches of each of the two teams.
3. **`top(k)` does not build the whole table.** It may hold at most
   `k + 1` rows at one time. The test: in a league of 2000 teams, `top(3)`
   may request at most 1024 bytes of heap memory in total.
4. **`fixtures()` does not build the list of pairings.** Creating the
   iterator may allocate at most once, and at most
   `n × size_of::<&str>()` bytes for `n` teams. `next`, `next_back`, `len`
   and `size_hint` must not allocate.

## 9. What is not specified

Anything this document does not mention is your decision. You may add
derives, trait implementations, private items and files. You may replace a
trait implementation whose body is `todo!()` by a derive. You may not
remove or change a public name, a public field, or a signature.
