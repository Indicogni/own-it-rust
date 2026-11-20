# Checkpoint 1 reference solution: commentary

The solution is `src/lib.rs` in this directory. Read it after your own
sitting, next to your code. This note explains the decisions that are not
obvious from the code, and what a weaker solution usually does instead.

## Money is an integer, and the grammar is checked by hand

`parse_amount` never converts to a floating-point number. `0.1 + 0.2` is
not `0.3` in an `f64`, and a ledger must add up to exactly zero.

The function checks the characters itself and uses `str::parse::<u64>` only
for the digits before the point, after it knows that they are digits. Two
reasons:

- `"+5".parse::<u64>()` is `Ok(5)`. The specification forbids a sign.
- Once the text is known to be digits, the only error `parse` can still
  return is an overflow. So the mapping to `TooLarge` is certain, and the
  code does not need to look inside the `ParseIntError`.

The check uses `bytes()` and `is_ascii_digit`, not `char::is_numeric`. The
second accepts digits of other scripts, such as `٣`, which `parse` then
rejects.

## One helper carries the parser

`next_word(&mut &'a str) -> Option<&'a str>` cuts one word off the front of
a string slice and moves the slice forward. Its signature has two lifetimes
on purpose. The word lives as long as the line (`'a`). The mutable borrow of
the local variable `rest` is much shorter. With one lifetime for both, the
second call would not compile.

`parse_entry` is then five short steps in the order of the specification,
and the order of the steps is the order in which errors win. The list of
sharers is cut with `split(',')` and `trim`, directly on the rest of the
line. Nothing is collected into a temporary list and nothing is copied, so
the function allocates once: when the first name is pushed.

The duplicate check is `sharers.contains(&name)`. That is quadratic in the
number of sharers. For a list that a person typed into one line, it is the
right choice: no extra collection, no allocation, and simple to read.

## The ledger is a sorted `Vec`

`Ledger` keeps `Vec<(String, i64)>` sorted by name. The invariant is
written on the type. It pays three times:

- `balance` is a binary search that compares `&str` with `&str`;
- `names` is already in order;
- `into_balances` returns the field. It is a move: no copy, no sort.

At this point of the course you have not met `HashMap` or `BTreeMap`
(chapter 16). A `BTreeMap<String, i64>` would also be a good answer here.
An unsorted `Vec` with a linear search passes all tests too, if
`into_balances` sorts it in place.

## `add` in two phases

The specification demands that a failed `add` changes nothing. There are
three ways to get that:

1. change the ledger, and undo the changes when something fails;
2. copy the ledger first and put the copy back on failure;
3. compute every new balance first, then write.

The solution uses the third. Phase 1 only reads `self` and fills a small
list of `(name, new balance)`. It may return early with `?`. Phase 2 only
writes and contains nothing that can fail. The first way is easy to get
wrong, and the second copies every name on every call.

The change of one person is computed in `i128`. A payer who shares their
own expense gets `+cents` and `-share`. The two can cancel out even when
`cents` alone is too large for an `i64`. Converting `cents` to `i64` first
rejects entries that the specification accepts. `i64::try_from` on the
final sum does the one range check that matters.

## Errors

The three small enums are `Copy` and hold no text. Their `Display`
implementations use `f.write_str` with fixed strings.

`EntryError::Amount` prints only `invalid amount` and returns the inner
`AmountError` from `source()`. A wrapper should either include the inner
message in its own, or expose the inner error as its source, not both. If
it did both, `report` would print the detail twice.

`read_ledger` builds the `ReadError` with a small closure that captures the
line number, and `map_err` applies it to both kinds of failure.

## `settle`

Three details matter.

- The sum is computed in `i128`. An `i64` sum of `i64::MIN + i64::MIN`
  panics in a debug build and is exactly `0` in a release build, which
  would wrongly report balanced input.
- The debt is `open[debtor].unsigned_abs()`. Writing `-open[debtor]`
  overflows for `i64::MIN`.
- The amount of a transfer is at most the credit, and a credit is at most
  `i64::MAX`. That is why the two updates cannot leave the range. The code
  says so in a comment and uses `wrapping_add_unsigned` and
  `wrapping_sub_unsigned`, which state the intent without a cast.

The search for the debtor and the creditor uses strict comparisons (`<` and
`>`), so among equal balances the first index stays. `Iterator::min_by_key`
would also keep the first minimum, but `max_by_key` keeps the *last*
maximum, which breaks the tie rule. The plain loop avoids the trap.

Every step makes at least one balance zero, so the loop ends after at most
`n - 1` transfers. Each step scans the whole list, so the plan costs
O(n²). For the size of a travel group that is the simplest correct choice.

## Tests in the solution

The `tests` module at the end tests the private helpers `next_word`,
`is_name`, `all_digits` and `checked_balance`, the remainder rule and the
sort invariant. They are the kind of test the rubric asks you to write
yourself: small, and written before the next function was started.
