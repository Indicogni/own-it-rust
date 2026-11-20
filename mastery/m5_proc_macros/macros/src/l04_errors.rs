//! LESSON 04 — Errors that point at the right place, and a compile-fail suite
//! Run: cargo test l04_
//! Read first: "Lesson 04" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A derive finds three problems in one enum. How many compiler runs should
//!    the user need to see all three?
//! 2. What does the user see when a macro calls `panic!` or `unwrap()` on bad
//!    input, and where does the compiler point?
//!
//! IMPLEMENT the tasks below. The error texts and the places they point at are
//! part of the contract: the unit tests check the line and column, and
//! `tests/ui/l04/*.stderr` holds what the compiler must print, character by
//! character. Then do task D: add a compile-fail case of your own.
//! EXPLAIN afterward: What does a `syn::Error` hold, and what does it become
//! in the macro's output? Why is a recorded `.stderr` file a test of your
//! spans and not only of your messages?
//!
//! COMPILER / TRANSFER LAB (optional): change one error from
//! `Error::new_spanned(..)` to `Error::new(Span::call_site(), ..)`, run
//! `cargo test l04_`, and read the difference that trybuild shows between the
//! expected and the actual output. Put the span back afterward.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Variant};

/// A. The label of one enum variant.
///
/// - `#[label("text")]` on the variant: that string literal, as the user wrote
///   it (the returned `LitStr` is the user's token, with its span).
/// - No `label` attribute: a new literal with the variant's name as its text
///   (without `r#`) and the span of the variant's name.
/// - Attributes with other names are ignored.
///
/// Errors. Every problem of the variant is reported, combined into one
/// `syn::Error`, in source order (attributes first, then the fields):
/// - the argument of `#[label(..)]` is not exactly one string literal: pass on
///   the error that `Attribute::parse_args::<LitStr>()` returns;
/// - a second `label` attribute, and each one after it:
///   ``duplicate `label` attribute`` spanned on the whole attribute
///   (`Error::new_spanned(attribute, ..)`). Its argument is not looked at;
/// - the variant has fields: ``` `Label` needs variants without fields ```
///   spanned on the fields, that is on `(u8)` or `{ by: u8 }`.
pub(crate) fn label_of(variant: &Variant) -> syn::Result<LitStr> {
    todo!("Implement this task")
}

/// B. Every variant of an enum with its label, in declaration order.
///
/// Errors. All of them are reported together, combined into one `syn::Error`:
/// - the input is a struct or a union: ``` `Label` can only be derived for enums ```
///   spanned on the keyword `struct` or `union`. This is the only error then.
/// - every error that task A finds, variant after variant;
/// - two variants with the same label text:
///   ``duplicate label `TEXT` (variant `FIRST` has it too)`` spanned on the
///   label of the later variant (the `LitStr` from task A). TEXT is the label's
///   text and FIRST is the name of the earliest variant with that label. A
///   variant that task A rejected has no label and takes no part in this check.
///
/// The combined error lists the problems in the order of the variants.
pub(crate) fn labels(input: &DeriveInput) -> syn::Result<Vec<(Ident, LitStr)>> {
    todo!("Implement this task")
}

/// C. `#[derive(Label)]`, from the raw input tokens to the output tokens.
/// This function must never panic, whatever the tokens are.
///
/// For `enum Level { Low, #[label("HIGH")] High }` the output is one impl
/// block of this shape (generic parameters and a where clause of the enum are
/// carried over as in lesson 03, without extra bounds):
///
/// ```text
/// impl ::m5_proc_macros::Label for Level {
///     fn label(&self) -> &'static str { /* "Low" or "HIGH" */ }
///     fn from_label(text: &str) -> ::core::option::Option<Self> { /* .. */ }
/// }
/// ```
///
/// `from_label` returns the variant whose label equals `text` exactly, or
/// `None`. An enum without variants is valid input.
///
/// On an error, the output is the error's `compile_error!` calls and nothing
/// else: no impl block. That covers the errors of task B and the case that
/// the tokens are not a struct, an enum or a union at all (pass on syn's
/// error).
pub(crate) fn label_impl(input: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

// D. Extend the compile-fail suite. Add a file `tests/ui/l04/own_NAME.rs`
// (choose NAME) with a use of `#[derive(Label)]` that must not compile and
// that the supplied cases do not cover yet, for example an attribute problem
// and a duplicate label in one enum, or a union. Run
//
//     TRYBUILD=overwrite cargo test l04_d
//
// once. trybuild writes what the compiler printed to
// `tests/ui/l04/own_NAME.stderr`. Read that file line by line before you keep
// it: from now on it is the expected output. The test `l04_d_own_case` in
// `tests/ui.rs` checks that both files exist and that the output still matches.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{
        assert_absolute, at, compact, errors_in, guess, is_clean, parse, predicates, tokens,
    };
    use syn::ItemImpl;

    fn input(source: &str) -> DeriveInput {
        parse(tokens(source), "a struct, enum or union")
    }

    /// The messages of an error with the position that each one points at.
    fn problems(error: syn::Error) -> Vec<(String, (usize, usize))> {
        error
            .into_iter()
            .map(|one| {
                let start = one.span().start();
                (one.to_string(), (start.line, start.column))
            })
            .collect()
    }

    fn variant(source: &str) -> Variant {
        parse(tokens(source), "an enum variant")
    }

    const NO_FIELDS: &str = "`Label` needs variants without fields";

    #[test]
    fn a_the_default_label_is_the_variant_name() {
        let source = "\n    Low";
        let label = label_of(&variant(source)).expect("a plain variant is valid");
        assert_eq!(label.value(), "Low");
        let start = label.span().start();
        assert_eq!(
            (start.line, start.column),
            at(source, "Low"),
            "a default label takes the span of the variant's name, so that a later error \
             about this label can point at the variant"
        );
        assert_eq!(
            label_of(&variant("r#Match")).expect("valid").value(),
            "Match",
            "the default label drops `r#`"
        );
    }

    #[test]
    fn a_the_attribute_sets_the_label() {
        let source =
            "/// Docs are attributes too.\n#[label(\"HIGH\")]\n#[allow(dead_code)]\nHigh = 7";
        let label = label_of(&variant(source)).expect("a labelled variant is valid");
        assert_eq!(label.value(), "HIGH");
        let start = label.span().start();
        assert_eq!(
            (start.line, start.column),
            at(source, "\"HIGH\""),
            "return the user's own literal token, not a new literal with the same text"
        );
    }

    #[test]
    fn a_fields_are_rejected_at_the_fields() {
        let source = "\n    High(u8)";
        let error = label_of(&variant(source)).expect_err("a variant with fields is invalid");
        assert_eq!(
            problems(error),
            [(NO_FIELDS.to_owned(), at(source, "(u8)"))],
            "the error must point at the fields, not at the variant's name or the derive"
        );
        let source = "\n    High { by: u8 }";
        let error = label_of(&variant(source)).expect_err("a variant with fields is invalid");
        assert_eq!(
            problems(error),
            [(NO_FIELDS.to_owned(), at(source, "{ by"))]
        );
    }

    #[test]
    fn a_bad_attribute_arguments_keep_syn_s_error() {
        let source = "\n    #[label(7)] Low";
        let error = label_of(&variant(source)).expect_err("7 is not a string literal");
        assert_eq!(
            problems(error),
            [("expected string literal".to_owned(), at(source, "7"))],
            "pass on the error from `parse_args::<LitStr>()`: it already points at the token"
        );
        assert!(label_of(&variant("#[label] Low")).is_err());
        assert!(label_of(&variant("#[label(\"a\", \"b\")] Low")).is_err());
        assert!(label_of(&variant("#[label = \"a\"] Low")).is_err());
    }

    #[test]
    fn a_every_problem_of_a_variant_is_reported() {
        let source =
            "\n    #[label(\"a\")]\n    #[label(\"b\")]\n    #[label(\"c\")]\n    High(u8)";
        let error = label_of(&variant(source)).expect_err("three problems");
        assert_eq!(
            problems(error),
            [
                (
                    "duplicate `label` attribute".to_owned(),
                    at(source, "#[label(\"b\")]")
                ),
                (
                    "duplicate `label` attribute".to_owned(),
                    at(source, "#[label(\"c\")]")
                ),
                (NO_FIELDS.to_owned(), at(source, "(u8)")),
            ],
            "combine all problems into one error, in source order. Stopping at the first one \
             makes the user compile once per mistake"
        );
    }

    #[test]
    fn b_variants_and_labels_in_declaration_order() {
        let found = labels(&input(
            "enum Level { Low, #[label(\"mid\")] Middle, #[label(\"HIGH\")] High }",
        ))
        .expect("a valid enum");
        let pairs: Vec<(String, String)> = found
            .iter()
            .map(|(variant, label)| (variant.to_string(), label.value()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("Low".to_owned(), "Low".to_owned()),
                ("Middle".to_owned(), "mid".to_owned()),
                ("High".to_owned(), "HIGH".to_owned())
            ]
        );
        assert!(
            labels(&input("enum Never {}")).expect("valid").is_empty(),
            "an enum without variants is valid and has no labels"
        );
    }

    #[test]
    fn b_structs_and_unions_are_rejected_at_the_keyword() {
        let source = "\n#[derive(Debug)]\npub struct Job { id: u32 }";
        let error = labels(&input(source)).expect_err("not an enum");
        assert_eq!(
            problems(error),
            [(
                "`Label` can only be derived for enums".to_owned(),
                at(source, "struct")
            )],
            "point at the keyword `struct`: that is the token that is wrong"
        );
        let source = "\nunion Bits { int: u32, float: f32 }";
        let error = labels(&input(source)).expect_err("not an enum");
        assert_eq!(
            problems(error),
            [(
                "`Label` can only be derived for enums".to_owned(),
                at(source, "union")
            )]
        );
    }

    #[test]
    fn b_duplicate_labels_point_at_the_later_label() {
        let source = "
enum Level {
    #[label(\"x\")]
    Low,
    High,
    #[label(\"x\")]
    Top,
    #[label(\"High\")]
    Peak,
    x,
}";
        let error = labels(&input(source)).expect_err("three duplicates");
        assert_eq!(
            problems(error),
            [
                (
                    "duplicate label `x` (variant `Low` has it too)".to_owned(),
                    at(source, "\"x\")]\n    Top")
                ),
                (
                    "duplicate label `High` (variant `High` has it too)".to_owned(),
                    at(source, "\"High\"")
                ),
                (
                    "duplicate label `x` (variant `Low` has it too)".to_owned(),
                    at(source, "x,\n}")
                ),
            ],
            "each duplicate points at the later label: the literal when there is an attribute, \
             the variant's name when the label is the default. FIRST is the earliest variant"
        );
    }

    #[test]
    fn b_all_problems_of_an_enum_are_reported_in_order() {
        let source = "
enum Level {
    Low(u8),
    #[label(\"same\")]
    Middle,
    #[label(7)]
    High,
    #[label(\"same\")]
    Top,
}";
        let error = labels(&input(source)).expect_err("three problems");
        assert_eq!(
            problems(error),
            [
                (NO_FIELDS.to_owned(), at(source, "(u8)")),
                ("expected string literal".to_owned(), at(source, "7")),
                (
                    "duplicate label `same` (variant `Middle` has it too)".to_owned(),
                    at(source, "\"same\")]\n    Top")
                ),
            ],
            "one compiler run must show every problem, in the order of the variants"
        );
    }

    #[test]
    fn c_the_output_is_one_impl_with_both_methods() {
        let output = label_impl(tokens(
            "enum Pick<T: Clone = u8> where T: Send { #[label(\"l\")] Left, Right }",
        ));
        assert!(
            is_clean(&output),
            "a valid enum must expand without errors: {output}"
        );
        let item: ItemImpl = parse(output.clone(), "one `impl` block");
        let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
        assert_eq!(compact(path), "::m5_proc_macros::Label");
        assert_eq!(compact(&item.generics.params), "T:Clone");
        assert_eq!(compact(&item.self_ty), "Pick<T>");
        assert_eq!(
            predicates(&item.generics),
            ["T:Send"],
            "the enum's where clause is copied, and nothing is added to it"
        );
        let methods: Vec<String> = item
            .items
            .iter()
            .filter_map(|item| match item {
                syn::ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(methods, ["label", "from_label"]);
        assert_absolute(
            &output,
            &["Option", "Some", "None", "core", "std", "m5_proc_macros"],
        );
    }

    #[test]
    fn c_errors_replace_the_impl() {
        let source = "\nenum Level {\n    Low(u8),\n    #[label(7)]\n    High,\n}";
        let output = label_impl(tokens(source));
        assert_eq!(
            errors_in(output.clone()),
            [
                (NO_FIELDS.to_owned(), at(source, "(u8)")),
                ("expected string literal".to_owned(), at(source, "7")),
            ],
            "the output must hold one `compile_error!` per problem, each with its own span"
        );
        assert!(
            !compact(&output).contains("impl"),
            "on an error, the output is the `compile_error!` calls only; output was: {output}"
        );
    }

    #[test]
    fn c_never_panics_on_strange_input() {
        for source in ["fn not_a_type() {}", "", "enum", "struct S { a: }"] {
            let output =
                std::panic::catch_unwind(|| label_impl(tokens(source))).unwrap_or_else(|_| {
                    panic!(
                        "label_impl panicked on `{source}`. A macro must turn every problem \
                         into a `compile_error!`: no `unwrap()`, no `expect()`, no `panic!`"
                    )
                });
            assert_eq!(
                errors_in(output).len(),
                1,
                "`{source}` is not an enum, struct or union: pass on syn's parse error"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        let source = "enum E {\n    High(u8),\n}";
        let item = input(source);
        let Data::Enum(data) = &item.data else {
            unreachable!("the source above is an enum")
        };
        let high = &data.variants[0];

        // P1. Where does an error start that is spanned on the fields of `High`?
        // Answer (line, column): lines count from 1, columns from 0.
        let on_fields = syn::Error::new_spanned(&high.fields, "first");
        let start = on_fields.span().start();
        let p1: (usize, usize) = guess();
        assert_eq!(
            p1,
            (start.line, start.column),
            "P1: start of the error (left: your prediction, right: actual)"
        );

        // P2. Three errors are combined. How many does iterating give back?
        let mut combined = on_fields;
        combined.combine(syn::Error::new_spanned(&high.ident, "second"));
        combined.combine(syn::Error::new_spanned(&item.ident, "third"));
        let p2: usize = guess();
        assert_eq!(
            p2,
            combined.clone().into_iter().count(),
            "P2: errors in the combined error (left: your prediction, right: actual)"
        );

        // P3. And what does `to_string()` of the combined error print?
        let p3: &str = guess();
        assert_eq!(
            p3,
            combined.to_string(),
            "P3: Display of a combined error (left: your prediction, right: actual)"
        );

        // P4. How many `compile_error!` calls does the combined error become?
        let p4: usize = guess();
        assert_eq!(
            p4,
            errors_in(combined.to_compile_error()).len(),
            "P4: compile_error! calls in the output (left: your prediction, right: actual)"
        );
    }
}
