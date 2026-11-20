# Checkpoint 3 rubric: `matchday`

[All checkpoints](../README.md) · [Specification](SPEC.md)

Use this file after your sitting has ended. It tells you which level you
reached and, for everything that did not work, which lesson to review.

There are two levels. Every criterion can be checked by running a command
or by reading your own code. Nothing depends on opinion.

## Level 1: pass

You pass when all of these are true.

- [ ] You worked closed-notes, in one sitting of at most 4 hours (see
      [the checkpoint rules](../README.md#rules)).
- [ ] `cargo test --test behaviour` passes.
- [ ] `cargo test --release --test behaviour` passes.
- [ ] The files in `tests/` are unchanged, and `Cargo.toml` has no
      dependencies.
- [ ] `grep -rn "unsafe" src/` finds nothing.

A pass means: you can give your own types the operators and standard
traits that callers expect, store closures, write an iterator by hand, and
pick collections that do the job, without help.

## Level 2: proficient

You are proficient when you pass and all of these are true as well.

- [ ] `cargo test` and `cargo test --release` pass completely. This adds
      `tests/technique.rs`.
- [ ] You deleted the `#![allow(dead_code, unused_variables, unreachable_code)]`
      line at the top of `src/lib.rs`, and `cargo clippy --all-targets`
      prints no warning. You added no other `allow` attribute.
- [ ] **One definition of each operation.** `+=` is written with `+` (or
      `+` with `+=`), `partial_cmp` is `Some(self.cmp(other))`, and one
      `Sum` implementation calls the other or both call the same helper.
      `grep -c "saturating_add" src/lib.rs` prints at most `5`.
- [ ] **The order is written once.** `grep -nE "sort_by|sort_unstable_by|max_by|min_by" src/`
      finds nothing: `standings` and `top` both use the `Ord` of `Row`.
- [ ] **Each collection has a reason.** For every collection type in your
      `League` and `Fixtures` you can complete the sentence "I chose it
      because the operation ... is ...", and the sentence is a comment at
      the field.
- [ ] **No copies of names.** `grep -nE "\.clone\(\)|to_string\(\)|to_owned\(\)|String::from" src/lib.rs`:
      outside your tests, the only hits are in error paths (the name inside
      a `DuplicateTeam` or `UnknownTeam`).
- [ ] **Invariants are written down.** The form of a team and the state of
      `Fixtures` each have a comment that states what is always true about
      them.
- [ ] `grep -nE "unwrap\(\)|expect\(|panic!|\[[a-z_.]+\]| as [ui](8|16|32|64|size)" src/lib.rs`:
      for every hit outside your tests you can say, in one sentence, why it
      cannot fail or lose information. The one planned panic is in `index`.
      If you cannot, it counts as not met.
- [ ] `src/lib.rs` has at least five tests of your own, and at least one of
      them tests a private function or type.

Proficient means: your types behave like the types of the standard
library. Their operators agree with each other, their order agrees with
their equality, lookups take a borrowed key, iterators are lazy, and the
memory the library uses depends on the number of teams and not on the
number of matches.

## If a test failed

Find the failed test in the first column. The names of related tests start
with the same word. "Chapter 15 · 10" means lesson 10 of chapter 15; read
its section in that chapter's `CONCEPTS.md` again, then repeat the lesson
and its drills.

### Behaviour tests

| failed tests | what they check | review |
|---|---|---|
| `outcome_not_is_the_view_of_the_other_team` | a unary operator for your own type | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 01 unary, binary and assignment operators |
| `score_from_a_pair`, `score_outcome_is_for_the_home_team`, `score_swapped_changes_the_side`, `tally_from_a_score_is_the_home_side`, `record_accepts_a_pair_as_score` | `From` for your own types, and a parameter that takes `impl Into<T>` | [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 06 infallible and fallible conversions |
| `tally_default_is_all_zero` | `Default` | [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 03 `Deref`, `DerefMut` and `Default` |
| `tally_add_and_add_assign` | `Add` and `AddAssign`, on a `Copy` type | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 01 operators; [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 02 `Clone` and `Copy` |
| `tally_fields_saturate`, `tally_played_and_goal_difference_are_exact` | an operator must choose what happens at the limit; a result can need a wider type than its inputs | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy; [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 01 operators |
| `tally_sum_of_values_and_of_references`, `standings_total_wins_equal_total_losses` | `Sum` for your own type, for items and for references to items | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 06 accumulators, · 08 folds |
| `row_more_points_is_smaller`, `row_ties_are_broken_in_the_order_of_the_spec`, `row_order_uses_the_points_of_the_row`, `row_goal_difference_is_compared_exactly`, `row_equality_and_sorting` | a hand-written `Ord` with several keys, some of them reversed | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 03 partial and total comparisons; [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 03 sorting |
| `new_accepts_str_and_string_from_any_iterator` | a generic parameter with `IntoIterator` and `Into<String>` | [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 06 conversions; [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 01 sources |
| `new_league_is_all_zero`, `new_league_may_be_empty`, `new_rejects_a_name_that_occurs_twice`, `teams_are_in_str_order` | a map from names to records: insert once, keep an order | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 06 hash maps, tree maps and entries |
| `record_updates_both_teams`, `record_the_example_of_the_spec`, `record_the_same_teams_may_meet_again`, `record_errors_in_the_order_of_the_spec`, `record_failure_changes_nothing` | changing two entries of one collection; checking everything before the first change | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 06 entries; [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 08 disjoint mutable borrows; [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 01 panics and invariants |
| `index_reads_a_tally`, `index_panics_for_an_unknown_team_and_names_it` | `Index` with a borrowed key | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 04 `Index` and `IndexMut` |
| `form_is_seen_from_each_team_oldest_first`, `form_keeps_the_last_five`, `form_reads_from_both_ends_and_knows_its_length` | a queue with a fixed length; returning an iterator that keeps the abilities of the collection's iterator | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 04 queues and ring buffers; [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 05 reversal, · 12 return types |
| `standings_of_the_example`, `standings_of_a_new_league_are_in_name_order`, `standings_break_ties_by_difference_then_goals` | building rows with an iterator chain and sorting them | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 03 mapping, · 09 collecting; [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 03 sorting |
| `standings_use_the_rule_of_the_league`, `rule_may_capture_and_return_large_numbers` | a closure stored in a struct and called later | [Chapter 14](../../exercises/chapter14/CONCEPTS.md) · 03 choosing callable bounds, · 04 erased closures |
| `top_is_the_start_of_the_standings` | the edge cases of a count: 0, more than there is, `usize::MAX` | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy; [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 05 priority queues |
| `observers_see_every_recorded_match`, `observers_run_in_order_and_may_keep_state`, `observers_are_not_called_for_a_failed_record` | a list of boxed `FnMut` callbacks | [Chapter 14](../../exercises/chapter14/CONCEPTS.md) · 02 mutation and move captures, · 06 callback registration and invocation |
| `fixtures_of_three_teams`, `fixtures_of_two_one_and_no_teams`, `fixtures_every_ordered_pair_once`, `fixtures_do_not_depend_on_recorded_matches`, `fixtures_can_be_recorded_while_the_names_are_kept` | `Iterator` for your own struct; the empty cases | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 02 generating sequences, · 10 implementing an iterator |
| `fixtures_len_counts_down` | `size_hint` and `ExactSizeIterator`; an iterator that stays finished | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 12 writing iterators: both ends and exact size |
| `fixtures_backwards`, `fixtures_from_both_ends_meet_in_the_middle` | `DoubleEndedIterator`: two ends that share one remaining range | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 10 implementing a double-ended iterator |
| `error_messages`, `errors_are_std_errors_without_source` | `Display` and `Error` for your own error types | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 06 custom errors |
| a test reports "was still running after 10 s" | an iterator that never returns `None` | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 10 implementing an iterator |

### Technique tests

| failed tests | what they check | review |
|---|---|---|
| `reading_the_league_does_not_allocate` | a map with `String` keys is searched with a `&str`; a method returns an iterator that borrows, not a new `Vec` | [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 05 `Borrow` and borrowed lookup; [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 12 return types |
| `recording_does_not_grow_the_league` | state that is bounded: totals instead of a history, and a ring of five instead of a growing list | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 04 queues and ring buffers, · 02 growth and removal |
| `top_holds_only_a_few_rows` | the best `k` of `n` items with a heap of `k + 1` items | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 05 priority queues and heap ordering |
| `top_of_everything_does_not_reserve_for_k` | a capacity that comes from an argument must be limited first | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 02 growth; [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy |
| `fixtures_are_computed_one_at_a_time` | an iterator computes each item when it is asked for; it does not store all items first | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 02 generating sequences, · 10 implementing an iterator |
| `rows_and_fixtures_borrow_the_names_of_the_league`, `rows_and_pairings_outlive_the_iterator_that_made_them` | an iterator whose items borrow from the collection (`'a`), not from the iterator itself | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 05 structs that borrow, · 06 independent lifetimes; [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 01 borrowed sources |
| `standings_ask_the_rule_once_for_each_team`, `top_asks_the_rule_once_for_each_team` | a caller's closure may be slow or may count its calls: compute a key once and store it, do not call it inside a comparison | [Chapter 14](../../exercises/chapter14/CONCEPTS.md) · 01 borrowed captures and sorting; [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 03 sorting |
| `the_rule_is_dropped_with_the_league` | a struct owns its boxed closures, and with them everything they captured | [Chapter 14](../../exercises/chapter14/CONCEPTS.md) · 04 erased closures, · 06 callbacks; [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 01 destruction |
| `row_order_agrees_with_equality` | `cmp` returns `Equal` exactly when `==` is true; collections such as `BTreeSet` rely on it | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 02 equivalence, · 03 partial and total comparisons |
| `row_order_is_transitive_over_many_rows` | comparing a difference of unsigned numbers without leaving their range | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 03 total comparisons; [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 02 integers and casts |
| the test program stops with "MEMORY LIMIT" | a loop or an iterator that never ends and stores something each time | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 04 stopping |

### A test passes in debug and fails with `--release`, or the other way round

Integer overflow panics in a debug build and wraps around in a release
build. Look at the additions in `Tally`, at the subtraction in the goal
difference, and at `k + 1` in `top`. Review
[Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03.

## If you ran out of time

| what happened | what it suggests | review |
|---|---|---|
| Most time went into the many small trait implementations at the top of the file | the signatures of the operator and conversion traits are not yet familiar | [Chapter 12](../../exercises/chapter12/CONCEPTS.md) · 01 and · 03; [Chapter 13](../../exercises/chapter13/CONCEPTS.md) · 06 |
| Most time went into compiler errors about closures: `Fn` against `FnMut`, `'static`, "cannot borrow as mutable" | storing and calling closures needs practice | [Chapter 14](../../exercises/chapter14/CONCEPTS.md) · 03, · 04 and · 06 |
| Most time went into `Fixtures` | you have not yet written enough iterators by hand; decide the state first, then write `next` | [Chapter 15](../../exercises/chapter15/CONCEPTS.md) · 10 and · 12 |
| You changed the fields of `League` several times | you chose the collections before you had listed the operations and their costs; read section 8 of the SPEC first next time | [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 04 to · 06, and the chapter's capstone from an empty file |
| Most time went into errors about two mutable borrows of the league | changing two entries of one map needs a plan | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 08; [Chapter 16](../../exercises/chapter16/CONCEPTS.md) · 06 |

## What to do with the result

- **Not passed:** review the lessons from the tables. Wait at least one
  week, then take the checkpoint again from the empty starter. Do not look
  at the reference solution in between.
- **Passed, not proficient:** read the reference solution and its
  commentary in `solutions/checkpoints/cp3_after_ch16/` of the course
  repository. For each technique test or rubric line that failed, change
  your own code until it holds. Then go on to chapter 17.
- **Proficient:** read the commentary anyway and compare the decisions
  with yours. Then go on to chapter 17.
