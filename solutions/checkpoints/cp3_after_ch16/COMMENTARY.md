# Checkpoint 3 reference solution: commentary

The solution is `src/lib.rs` in this directory. Read it after your own
sitting, next to your code. This note explains the decisions that are not
obvious from the code, and what a weaker solution usually does instead.

## Choose the collections from the operations

Section 8 of the specification is a list of costs. Read as a list of
operations, it almost names the fields:

| operation | what it needs | choice |
|---|---|---|
| find a team by `&str`, without allocating | a key type that can be searched with a borrowed form | a map with `String` keys |
| list the names in `str` order, without allocating | keys that are kept in order | `BTreeMap`, not `HashMap` |
| keep the last five outcomes | remove at the front, add at the back | `VecDeque` with a fixed capacity |
| best `k` of `n` rows with `k + 1` rows of memory | "remove the worst of these" | `BinaryHeap` |
| the pairing at a position | access by index | `Vec<&str>` inside `Fixtures` |

`BTreeMap<String, Club>` gives the first two for free. `get(team)` takes a
`&str` because `String` implements `Borrow<str>`, and `str` has the same
order as `String`. Writing `get(&team.to_string())` also compiles, and it
allocates on every lookup. The technique tests catch that.

A `HashMap<String, Club>` is a fine choice for the lookups. It needs a
sorted list of names for `teams()` as well, so the names are stored twice
or sorted on demand. A sorted `Vec` with a binary search works too.

## Nothing is stored per match

The league keeps totals, not history. A match changes two tallies and two
forms and is then forgotten. That is why 2000 more matches allocate
nothing.

The form is a ring. `Club::new` creates the `VecDeque` with capacity 5, and
`Club::add` removes the oldest outcome before it adds the sixth. A `Vec`
that only grows, with `form()` returning its last five elements, gives the
same answers and fails `recording_does_not_grow_the_league`.

## `record` checks first, then changes

The three checks only read. The changes come after the last check and
contain nothing that can fail. So an `Err` leaves the league exactly as it
was, and no "undo" code exists.

The two teams are changed one after the other:

```text
for (team, seen) in [(home, score), (away, score.swapped())] {
    if let Some(club) = self.clubs.get_mut(team) { club.add(seen); }
}
```

Each `get_mut` borrow ends before the next one starts, so the borrow
checker has no objection. Holding both `&mut Club` at once would need more
machinery and gains nothing here. The `if let` cannot take its `else`
path, because phase 1 has seen both keys. It is written this way so that
the function has no `unwrap`.

## Small operator implementations that agree with each other

- `AddAssign` is `*self = *self + other`. One definition of addition
  exists, in `Add`. If the two were written separately, one of them could
  saturate and the other one wrap.
- `Sum<&Tally>` calls `Sum<Tally>` through `copied()`, and that one is a
  `fold` with `Add::add`.
- `Add` uses `saturating_add` because the specification chooses
  saturation. Plain `+` panics in a debug build and wraps in a release
  build: two behaviours, and neither is the specified one.
- `played()` returns `u64` and `goal_difference()` returns `i64`. The
  operands are widened before the operation. Three `u32` values can add up
  to more than a `u32` holds, and a difference of two unsigned numbers can
  be negative.

`Tally::from(score)` uses `u32::from(outcome == Outcome::Win)`. A `bool`
converts to 0 or 1, which avoids three `match` arms that differ in one
digit.

## The order of the table is the `Ord` of `Row`

The ranking is written once, in `Row::cmp`, as a chain of `then_with`.
Where more is better, `other` is on the left side of the comparison. The
name is compared the normal way round.

Three things follow from making this the `Ord` of the type, instead of a
closure that is passed to `sort_by`:

- `standings` is `collect` and `sort_unstable`;
- `top` can put rows into a `BinaryHeap`, which needs `Ord`;
- callers can use `min()`, `BTreeSet<Row>` or `rows.is_sorted()`.

**The last key.** The table is decided after the name: two rows of one
league never have the same name. But `Row` has public fields, so anyone can
build two rows with the same name, points, difference and goals, and with
different numbers of wins. The derived `==` says they are different. If
`cmp` said `Equal`, a `BTreeSet<Row>` would silently drop one of them.
The rule "`cmp` is `Equal` exactly when `==` is true" is part of the
contract of `Ord`. The fifth key, `(won, drawn, lost)`, exists only to
keep it. Goals against need no key of their own: equal goals for and an
equal difference already force them to be equal.

`partial_cmp` is `Some(self.cmp(other))`. It must never be a second copy of
the logic.

The goal difference is compared as `i64`. Comparing
`goals_for - goals_against` in `u32` panics or wraps as soon as a team has
conceded more than it has scored.

## The rule runs when a row is made

`rows()` is a private iterator that builds one `Row` per team and calls the
rule once for it. `standings` and `top` both start from `rows()`. The
points are then a plain number inside the row, and sorting never calls the
closure again.

The tempting alternative is `rows.sort_by_key(|row| (self.rule)(&row.tally))`
or a comparison closure that computes points. A sort compares each element
many times, so the caller's closure runs many times per team. For a rule
that is slow, or that counts its calls, that is a visible difference.

The rule is stored as `Box<dyn Fn(&Tally) -> u64>` and the observers as
`Vec<Box<dyn FnMut(&str, &str, Score)>>`. The league is not generic over
the closure types: a generic parameter would appear in the type of every
league, and one league could not hold observers of different types.
`with_rule` takes `self` and returns it, so a rule can be set in the same
expression that creates the league.

## `top` with a heap of `k + 1` rows

`top(k)` pushes every row into a max-heap and removes the top whenever the
heap has more than `k` rows. Because "greater" means "lower in the table",
the top of the heap is always the worst row that is still kept. At the end
`into_sorted_vec` sorts in place, in ascending order, which is table order.

Two details:

- `k` is limited to the number of teams before anything is reserved.
  `top(usize::MAX)` is a valid call. `with_capacity(k + 1)` with the
  caller's `k` would overflow, or ask for an absurd amount of memory.
- No `Reverse` is needed. It would be needed if the order of `Row` were
  "greater is better".

For a table of twenty teams the difference to "sort everything and
truncate" does not matter. It matters when `n` is large and `k` is small,
and the pattern is worth knowing by heart.

## `Fixtures` holds a range of numbers

There are `n × (n − 1)` pairings. The iterator numbers them in the order in
which it yields them and stores only two numbers: the next one from the
front and the end. `pairing(number)` computes the pair:

```text
guests = n - 1                 each team is at home `guests` times in a row
home   = number / guests
slot   = number % guests       position among the guests
away   = slot, or slot + 1 from the home team's own position on
```

With this state, `next_back` is three lines, `size_hint` is a subtraction,
and the two ends cannot cross because they share one range. An iterator
that keeps a `(home, away)` position for the front needs a second such
pair for the back and a careful test for the moment they meet.

`end` is computed with `saturating_sub(1)`, so that a league with no teams
gives 0 and not an underflow. `pairing` divides by `n − 1`. It is only
called when a pairing is left, which implies at least two teams.

The item type is `(&'a str, &'a str)` with the lifetime of the league, not
of the iterator. That is why a pair can be kept after the iterator is
gone.

`ExactSizeIterator` has no required method. Its `len` relies on
`size_hint`, and the default `size_hint` returns `(0, None)`. An
implementation that forgets to override `size_hint` compiles, and `len()`
then panics.

## `teams` and `form` return `impl Iterator`

Both return the iterator of the underlying collection with one adapter:
`keys().map(String::as_str)` and `iter().copied()`. No named type and no
allocation are needed. `VecDeque`'s iterator is double-ended and knows its
length, and `copied` keeps both abilities, so the bounds in the signature
of `form` hold without any code.

## Tests in the solution

The `tests` module checks the private parts directly: the numbering of
pairings, the ring of the form, the side from which a club sees a score,
the order of `rows()`, and the direction of the heap. Each of them was a
place where a wrong assumption would have cost time later.
