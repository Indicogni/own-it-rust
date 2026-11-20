# Checkpoint 1 rubric: `splitpot`

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
- [ ] `grep -n "unsafe" src/` finds nothing.

A pass means: you can turn a written specification into a correct Rust
program with the material of chapters 3 to 7, without help.

## Level 2: proficient

You are proficient when you pass and all of these are true as well.

- [ ] `cargo test` and `cargo test --release` pass completely. This adds
      `tests/technique.rs`.
- [ ] You deleted the `#![allow(dead_code, unused_variables)]` line at the
      top of `src/lib.rs`, and `cargo clippy --all-targets` prints no
      warning. You added no other `allow` attribute.
- [ ] `grep -nE "f32|f64" src/` finds nothing.
- [ ] `grep -nE "unwrap\(\)|expect\(|panic!|\[[a-z_]+\]" src/lib.rs`: for
      every hit outside your tests you can say, in one sentence, why it
      cannot fail for any input. If you cannot, it counts as not met.
- [ ] `grep -nE "\.clone\(\)|to_string\(\)|to_owned\(\)|String::from" src/lib.rs`:
      outside your tests and outside `report`, the only place that copies
      text is the one where a new name enters the ledger.
- [ ] `src/lib.rs` has at least five tests of your own, and at least one of
      them tests a private helper function.
- [ ] `Ledger` has a comment that states its invariant (what is always true
      about its fields), and `add` has a comment that says why a failure
      leaves the ledger unchanged.

Proficient means: the program is correct for reasons you chose on purpose.
It borrows where borrowing is enough, its arithmetic is as wide as the
values need, and a failed operation leaves no trace.

## If a test failed

Find the failed test in the first column. The names of related tests start
with the same word. "Chapter 5 · 04" means lesson 04 of chapter 5; read its
section in that chapter's `CONCEPTS.md` again, then repeat the lesson and
its drills.

### Behaviour tests

| failed tests | what they check | review |
|---|---|---|
| `amount_whole_numbers_and_decimals`, `amount_leading_zeros_are_allowed` | reading digits into an integer number of cents | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 02 integers, · 10 UTF-8 bytes and string slices |
| `amount_the_largest_value_is_u64_max_cents` | an overflow is found, not wrapped and not a panic | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy, · 14 checked conversions |
| `amount_empty_text`, `amount_malformed_texts` | the whole text is checked against the grammar; `str::parse` alone accepts `+5` | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 14 checked conversions and `str::parse`, · 10 Unicode |
| `entry_fields`, `entry_white_space_is_flexible`, `entry_names_are_any_text_without_white_space_or_commas` | cutting a line into borrowed pieces | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 09 slices, · 10 string slices; [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 04 lifetimes through functions |
| `entry_syntax_errors`, `entry_duplicate_sharer` | every way a line can be wrong ends in an `Err`, never in a panic | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 03 `if let` and `let else`; [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 01 panics and invariants |
| `entry_amount_errors_keep_the_reason`, `entry_the_first_problem_from_the_left_decides`, `error_conversion_from_amount_error` | an inner error is kept inside the outer one; `?` converts it | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 03 the `?` operator, · 05 multiple error types |
| `ledger_starts_empty`, `ledger_one_entry`, `ledger_payer_who_does_not_share`, `ledger_entries_add_up` | a collection of owned records that is searched and updated | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 08 vectors; [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 07 sharing versus mutation |
| `ledger_left_over_cents_go_to_the_first_sharers`, `ledger_balances_always_add_up_to_zero`, `ledger_entry_without_sharers_changes_nothing` | integer division with a remainder; division by zero | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 02 integers, · 14 numeric edge cases |
| `ledger_names_and_balances_are_sorted_by_name`, `ledger_zero_amount_still_introduces_the_people` | sorting text; `str` order is byte order | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 08 vectors, · 10 Unicode |
| `ledger_overflow_is_reported` | checked arithmetic on balances | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy |
| `file_with_comments_and_blank_lines`, `file_empty_and_comment_only`, `file_windows_line_endings` | a loop that skips, continues and stops | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 04 loops, `break` and `continue` |
| `file_errors_carry_the_line_number`, `file_overflow_carries_the_line_number` | adding context to an error on its way up | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 02 catching errors and printing context, · 04 `Result` methods |
| `error_messages` | `Display` for your own error types | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 06 custom errors |
| `error_sources_form_a_chain`, `report_joins_the_chain` | `Error::source`, and walking the chain | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 06 custom errors with a source chain |
| `errors_work_with_question_mark_and_boxed_errors` | your errors convert into `Box<dyn Error + Send + Sync>` | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 05 boxed errors, · 09 thread-safe boxed errors |
| `settle_nothing_to_do`, `settle_two_people`, `settle_the_trip`, `settle_largest_debt_pays_largest_credit_first`, `settle_ties_go_to_the_earlier_position`, `settle_plan_brings_every_balance_to_zero` | a loop with a state that must reach an end; finding a minimum and a maximum with a tie rule | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 04 loops, · 05 loop values and labels |
| `settle_unbalanced_input` | checking a precondition and reporting it as an error value | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 01 panics and invariants |
| a test reports "was still running after 10 s" | a loop whose state does not change | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 04 loops |

### Technique tests

| failed tests | what they check | review |
|---|---|---|
| `entry_names_are_slices_of_the_line`, `entries_can_outlive_the_loop_that_reads_them` | the result borrows from the input, with the input's lifetime | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 04 lifetimes through functions, · 05 structs that borrow, · 06 independent lifetimes |
| `parse_amount_never_allocates`, `parse_entry_allocates_only_the_list_of_sharers`, `rejected_lines_cost_no_allocation` | work is done on slices of the input; no temporary `String` or `Vec` | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 09 slices, · 11 building text; [Chapter 4](../../exercises/chapter04/CONCEPTS.md) · 06 `Copy` and explicit cloning |
| `looking_up_a_balance_does_not_allocate` | comparing a `String` with a `&str` needs no new `String` | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 01 borrowing and dereferencing |
| `into_balances_moves_the_names` | a method that takes `self` can give its parts away | [Chapter 4](../../exercises/chapter04/CONCEPTS.md) · 02 moves across fields, · 04 moving out of collections, · 05 leaving a replacement behind |
| `add_does_not_copy_the_ledger`, `failed_add_does_not_credit_the_payer`, `failed_add_does_not_introduce_new_people` | validate everything, then change; a failure leaves no partial update | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 01 panics and invariants, · 07 deliberate recovery; [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 07 sharing versus mutation |
| `credit_and_own_share_are_combined_before_the_range_check` | an intermediate value needs a wider type than the result | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 02 integers and casts, · 14 checked conversions (`TryFrom`) |
| `settle_handles_a_debt_of_i64_min` | `-x` and `x.abs()` overflow for the lowest value of a signed type | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 14 numeric edge cases, · 03 overflow policy |
| `settle_checks_the_exact_sum` | a sum can wrap around to a wrong answer in release builds and panic in debug builds | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy |
| `transfers_borrow_the_names_of_the_balances` | a result struct that borrows from an argument | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 05 structs that borrow |
| the test program stops with "MEMORY LIMIT" | a loop that never ends and pushes to a `Vec` each time | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 04 loops |

### A test passes in debug and fails with `--release`, or the other way round

Integer overflow panics in a debug build and wraps around in a release
build. A difference between the two profiles almost always means that an
addition, a subtraction, a negation or an `as` cast leaves its range
somewhere. Review [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03.

## If you ran out of time

| what happened | what it suggests | review |
|---|---|---|
| Most time went into compiler errors about borrows or lifetimes | the ownership model is not yet automatic | [Chapter 4](../../exercises/chapter04/CONCEPTS.md) and [Chapter 5](../../exercises/chapter05/CONCEPTS.md): repeat the drills of both chapters |
| Most time went into the parser | string slicing and `Option` handling need practice | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 09 to 11; [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 03 |
| Most time went into the error types | the `Error` trait is not yet familiar | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 05 and · 06 |
| You wrote everything, then debugged for an hour | you tested too late; write a test of your own after each function | [Chapter 2](../../exercises/chapter02/CONCEPTS.md) · 09 writing your own tests |

## What to do with the result

- **Not passed:** review the lessons from the tables. Wait at least one
  week, then take the checkpoint again from the empty starter. Do not look
  at the reference solution in between.
- **Passed, not proficient:** read the reference solution and its
  commentary in `solutions/checkpoints/cp1_after_ch07/` of the course
  repository. For each technique test that failed, change your own code
  until it passes. Then go on to chapter 8.
- **Proficient:** read the commentary anyway and compare the decisions
  with yours. Then go on to chapter 8.
