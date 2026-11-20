# Checkpoint 2 rubric: `postage`

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

A pass means: you can model a small domain with structs, enums and traits,
and get every rule of a written specification right, without help.

## Level 2: proficient

You are proficient when you pass and all of these are true as well.

- [ ] `cargo test` and `cargo test --release` pass completely. This adds
      `tests/technique.rs`.
- [ ] You deleted the `#![allow(dead_code, unused_variables)]` line at the
      top of `src/lib.rs`, and `cargo clippy --all-targets` prints no
      warning. You added no other `allow` attribute.
- [ ] **Files.** Each of the three modules is in its own file (or its own
      directory), and `grep -cE "^\s*(pub )?(fn|struct|enum|trait|impl)" src/lib.rs`
      prints `0`: the crate root only declares modules and re-exports.
- [ ] **Dependencies point one way.** `grep -nE "carrier|quote" src/parcel.rs`
      and `grep -n "quote" src/carrier.rs` find nothing outside comments.
- [ ] **Private fields.** `grep -rnE "^\s*pub [a-z_]+:" src/` finds only
      the two fields of `Quote`.
- [ ] **Derived comparisons.** `grep -rnE "impl (PartialEq|Eq|PartialOrd|Ord|Hash) for" src/`
      finds nothing. You chose the fields and the order of the variants so
      that the derived implementations are the correct ones.
- [ ] **Patterns.** `Parcel::size_class` and `Snail::price` contain no
      `else if`. Each row of a SPEC table is one `match` arm.
- [ ] **Invariants are written down.** `Dimensions` has a comment that
      states what is always true about its fields.
- [ ] `grep -rnE "unwrap\(\)|expect\(|panic!| as [ui](8|16|32|64|size)" src/`:
      for every hit outside your tests you can say, in one sentence, why it
      cannot fail or lose information. If you cannot, it counts as not met.
- [ ] You wrote at least five tests of your own, and every module has at
      least one.

Proficient means: the types make wrong values impossible to build, the
compiler writes the comparisons for you because you chose the
representation well, and the trait works equally with static and with
dynamic dispatch.

## If a test failed

Find the failed test in the first column. "Chapter 9 · 02" means lesson 02
of chapter 9; read its section in that chapter's `CONCEPTS.md` again, then
repeat the lesson and its drills.

### Behaviour tests

| failed tests | what they check | review |
|---|---|---|
| `grams_wrap_a_number`, `grams_compare_like_numbers` | a tuple struct around one number, with derived traits | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 02 tuple structs, · 07 deriving common traits |
| `dimensions_are_sorted_whatever_the_argument_order`, `dimensions_reject_sides_outside_the_range`, `volume_is_the_product_of_the_sides` | a constructor that checks its arguments and stores a normal form | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 03 methods and associated items, · 09 capstone (constructor invariants) |
| `a_box_turned_on_its_side_is_the_same_box` | derived `==` is field by field, so the fields must be in a normal form | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 07 deriving common traits |
| `parcel_keeps_what_it_was_given`, `parcel_weight_must_be_in_range`, `parcel_liquid_must_fit_in_the_box` | private fields with accessors; an enum variant with a payload | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 01 named structs; [Chapter 10](../../exercises/chapter10/CONCEPTS.md) · 01 variants and payloads |
| `parcel_error_messages` | `Display` and `Error` for an error enum | [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 06 custom errors |
| `billable_weight_is_the_greater_of_weight_and_volume` | integer division that rounds up; comparing two values of your own type | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 02 integers; [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 07 deriving `Ord` |
| `size_class_order` | the derived order of an enum is the order of its variants | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 07 deriving common traits; [Chapter 10](../../exercises/chapter10/CONCEPTS.md) · 01 variants |
| `size_class_of_the_sample_parcels`, `size_class_letter_limits`, `size_class_small_and_medium_limits` | a table of conditions as a `match` with ranges, where the first arm that matches wins | [Chapter 10](../../exercises/chapter10/CONCEPTS.md) · 04 literal, tuple and struct patterns, · 06 array and slice patterns, · 07 guards and alternatives |
| `snail_price_table`, `snail_adds_a_surcharge_for_fragile_goods`, `snail_takes_liquid_only_inside_the_country` | matching on a pair of enums, with a nested enum | [Chapter 10](../../exercises/chapter10/CONCEPTS.md) · 04 tuple patterns, · 07 alternatives |
| `accepts_follows_the_price_by_default` | a provided method in a trait | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 01 traits, implementations and defaults |
| `dash_charges_by_started_kilogram`, `dash_multiplies_by_zone`, `dash_takes_liquid_only_locally`, `dash_keeps_its_own_name` | implementing a trait for a struct with data | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 01 traits and implementations; [Chapter 4](../../exercises/chapter04/CONCEPTS.md) · 06 explicit cloning |
| `dash_declines_when_the_price_does_not_fit` | checked arithmetic that ends in `None` | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy; [Chapter 7](../../exercises/chapter07/CONCEPTS.md) · 03 the `?` operator (it works on `Option` too) |
| `discount_reduces_the_price_and_rounds_up`, `discount_limits`, `discount_keeps_the_name_and_the_refusals`, `discounts_can_be_stacked` | a generic struct, and a trait implemented for it with a bound | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 05 generic structs and impl blocks; [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 01 implementations |
| `cheapest_picks_the_lowest_price`, `cheapest_skips_carriers_that_decline`, `cheapest_prefers_the_earlier_carrier_among_equal_prices`, `cheapest_handles_a_free_carrier` | a loop over trait objects that keeps the best one | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 02 static and dynamic dispatch; [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 03 `let else` |
| `total_adds_the_prices`, `total_works_with_and_without_dynamic_dispatch`, `total_does_not_overflow` | one generic function for sized types and for `dyn Trait` | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 02 dispatch, · 06 bounds |
| `the_root_re_exports_the_same_items` | module paths and `pub use` | [Chapter 8](../../exercises/chapter08/CONCEPTS.md) · 02 nested modules, · 03 paths, imports and re-exports |

### Technique tests

| failed tests | what they check | review |
|---|---|---|
| `a_newtype_costs_nothing`, `value_types_are_small` | the size of a struct is the size of its fields plus padding; store facts, compute the rest | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 02 tuple structs, unit structs and layout |
| `volume_of_the_largest_box_needs_64_bits`, `liquid_check_does_not_overflow`, `discount_does_not_overflow` | widen the operands before a multiplication, not the result after it | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03 overflow policy, · 14 checked conversions |
| `building_and_classifying_a_parcel_does_not_allocate`, `pricing_does_not_allocate` | small `Copy` values need no heap; a name is returned as `&str` | [Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 07 arrays, · 08 vectors; [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 04 lifetimes through functions |
| `comparing_carriers_does_not_allocate` | keep the best candidate in a variable instead of collecting all candidates | [Chapter 6](../../exercises/chapter06/CONCEPTS.md) · 04 loops |
| `a_quote_borrows_the_winning_carrier` | a struct that holds `&dyn Trait`; a tie must keep the earlier candidate | [Chapter 5](../../exercises/chapter05/CONCEPTS.md) · 02 reference identity, · 05 structs that borrow; [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 02 dynamic dispatch |
| `discounted_gives_its_carrier_back` | a generic wrapper owns its value and can move it out again | [Chapter 4](../../exercises/chapter04/CONCEPTS.md) · 02 moves across fields; [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 05 generic structs |
| `cheapest_asks_each_carrier_for_one_price` | a provided method calls a required one, so calling both does the work twice | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 01 defaults |
| `a_box_forwards_every_method`, `a_discount_forwards_accepts` | an implementation for a wrapper must forward provided methods too, or an override inside is lost | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 01 defaults, · 04 extension traits and qualified calls |
| `wrappers_work_through_dynamic_dispatch` | `impl<C: Carrier + ?Sized> Carrier for Box<C>`; a generic type with a trait object inside | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 02 dispatch, · 06 bounds |

### A test passes in debug and fails with `--release`, or the other way round

Integer overflow panics in a debug build and wraps around in a release
build. Look for a multiplication of two `u32` values. Review
[Chapter 3](../../exercises/chapter03/CONCEPTS.md) · 03.

## If you ran out of time

| what happened | what it suggests | review |
|---|---|---|
| Most time went into the two tables (`size_class`, `Snail::price`) | you wrote chains of `if` where one `match` on a tuple would do | [Chapter 10](../../exercises/chapter10/CONCEPTS.md) · 04 and · 07 |
| Most time went into compiler errors about `dyn Carrier`, `Sized` or lifetimes of trait objects | dispatch and bounds are not yet familiar | [Chapter 11](../../exercises/chapter11/CONCEPTS.md) · 02 and · 06 |
| Most time went into "private field" and "unresolved import" errors | module visibility and paths need practice | [Chapter 8](../../exercises/chapter08/CONCEPTS.md) · 02 and · 03 |
| You could not decide which fields to give the structs | practise designing from the operations: list what must be cheap, then choose | [Chapter 9](../../exercises/chapter09/CONCEPTS.md) · 09 capstone, from an empty file |

## What to do with the result

- **Not passed:** review the lessons from the tables. Wait at least one
  week, then take the checkpoint again from the empty starter. Do not look
  at the reference solution in between.
- **Passed, not proficient:** read the reference solution and its
  commentary in `solutions/checkpoints/cp2_after_ch11/` of the course
  repository. For each technique test or rubric line that failed, change
  your own code until it holds. Then go on to chapter 12.
- **Proficient:** read the commentary anyway and compare the decisions
  with yours. Then go on to chapter 12.
