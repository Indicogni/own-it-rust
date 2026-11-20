//! LESSON 03 — Generics, where clauses and helper attributes
//! Run: cargo test l03_
//! Read first: "Lesson 03" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. `struct Wrapper<T = u8>` has a default for `T`. Where in
//!    `impl<..> Trait for Wrapper<..>` may that default be written?
//! 2. A masked field is never printed. Should its type still have to
//!    implement `Debug`?
//!
//! IMPLEMENT the tasks below. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: What are the three pieces that `split_for_impl` returns,
//! and where does each go? Compare "one bound per type parameter" with "one
//! bound per field type": give one struct for which each choice is wrong or
//! too strict.
//!
//! COMPILER / TRANSFER LAB (optional): write `#generics` in both places of the
//! `impl` header instead of the pieces from `split_for_impl`, and run
//! `cargo test l03_`. Read the compiler's messages in the trybuild output for
//! the struct with a default type parameter and for the struct with a bound.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Field, Fields, Generics, Type, WhereClause};

/// How one field appears in the redacted text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Printed with `{:?}`.
    Show,
    /// Printed as `***`.
    Mask,
    /// Left out.
    Skip,
}

/// What the helper attributes say about one field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    /// The name printed in front of the value.
    pub label: String,
    pub mode: Mode,
}

/// A. Read the `#[redacted(...)]` attributes of one field.
///
/// - No such attribute: `Mode::Show`, and the label is the field's name
///   (without `r#`).
/// - `#[redacted(mask)]` selects `Mode::Mask`, `#[redacted(skip)]` selects
///   `Mode::Skip`. When both are given, the later one wins.
/// - `#[redacted(rename = "text")]` sets the label to `text`.
/// - One attribute may hold several arguments, separated by commas, and a
///   field may have several `#[redacted(...)]` attributes. All of them apply.
/// - Attributes with any other name (`#[doc = ".."]`, `#[serde(..)]`) belong
///   to someone else: ignore them.
/// - Any other argument, or `#[redacted]` without parentheses, is an `Err`.
///   The error text is free in this lesson; lesson 04 is about good errors.
pub(crate) fn plan_field(field: &Field) -> syn::Result<Plan> {
    todo!("Implement this task")
}

/// B. The where clause for the generated impl: every predicate of the
/// struct's own where clause, in order, and then one predicate
/// `TYPE: ::core::fmt::Debug` for each type in `shown`, in order.
/// Return `None` when there is no predicate at all.
///
/// The tokens of `::core::fmt::Debug` are new, so you choose their span. Give
/// them the span of TYPE (`syn::spanned::Spanned::span`, with `quote_spanned!`
/// or `parse_quote_spanned!`). Then the compiler reports a field type without
/// `Debug` at that field, and not at the `#[derive(...)]` line.
pub(crate) fn where_with_debug(generics: &Generics, shown: &[&Type]) -> Option<WhereClause> {
    todo!("Implement this task")
}

/// C. `#[derive(Redacted)]` for a struct with named fields, with or without
/// generic parameters. For
///
/// ```text
/// struct Login<'a, T> where T: Clone {
///     user: &'a str,
///     #[redacted(mask)]
///     password: T,
///     #[redacted(skip)]
///     attempts: u32,
/// }
/// ```
///
/// the output is one impl block of this shape (the body is yours to design):
///
/// ```text
/// impl<'a, T> ::m5_proc_macros::Redacted for Login<'a, T>
/// where
///     T: Clone,
///     &'a str: ::core::fmt::Debug,
/// {
///     fn redacted(&self) -> ::std::string::String { /* .. */ }
/// }
/// ```
///
/// and `redacted()` returns `Login { user: "ann", password: *** }`:
/// the struct's name, then the fields that are not skipped, in declaration
/// order, as `label: value` separated by `, `. A shown value is printed with
/// `{:?}`, a masked value as `***`. When no field is printed, the text is
/// `Login {}`.
///
/// The where clause is the one from task B, where `shown` holds the types of
/// the fields with `Mode::Show`. A masked or skipped field adds no bound: the
/// struct above implements `Redacted` for a `T` without `Debug`.
///
/// Return `Err` for any input that is not a struct with named fields, and
/// pass on an `Err` from task A.
pub(crate) fn redacted_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{assert_absolute, at, compact, flatten, guess, parse, tokens};
    use proc_macro2::TokenTree;
    use syn::ItemImpl;

    fn input(source: &str) -> DeriveInput {
        parse(tokens(source), "a struct, enum or union")
    }

    fn field(source: &str) -> Field {
        syn::parse::Parser::parse2(Field::parse_named, tokens(source))
            .unwrap_or_else(|error| panic!("test input is not a named field: {error}"))
    }

    fn plan(source: &str) -> Plan {
        plan_field(&field(source))
            .unwrap_or_else(|error| panic!("`{source}` should be accepted, but: {error}"))
    }

    const VIEW: &str = "
struct View<'a, T: Clone = u8, const N: usize = 4>
where
    T: Send,
{
    title: &'a str,
    #[redacted(mask)]
    secret: T,
    rows: [u8; N],
    #[redacted(skip)]
    cache: Vec<T>,
}";

    #[test]
    fn a_a_field_without_attributes_is_shown_under_its_name() {
        assert_eq!(
            plan("pub user: String"),
            Plan {
                label: "user".into(),
                mode: Mode::Show
            }
        );
        assert_eq!(plan("r#type: u8").label, "type", "the label drops `r#`");
    }

    #[test]
    fn a_mask_skip_and_rename_are_read() {
        assert_eq!(plan("#[redacted(mask)] password: String").mode, Mode::Mask);
        assert_eq!(plan("#[redacted(skip)] cache: Vec<u8>").mode, Mode::Skip);
        assert_eq!(
            plan("#[redacted(rename = \"login name\")] user: String"),
            Plan {
                label: "login name".into(),
                mode: Mode::Show
            }
        );
    }

    #[test]
    fn a_arguments_and_attributes_add_up() {
        let expected = Plan {
            label: "pw".into(),
            mode: Mode::Mask,
        };
        assert_eq!(
            plan("#[redacted(mask, rename = \"pw\")] password: String"),
            expected,
            "two arguments in one attribute"
        );
        assert_eq!(
            plan("#[redacted(rename = \"pw\",)] #[redacted(mask)] password: String"),
            expected,
            "two attributes on one field, and a trailing comma"
        );
        assert_eq!(
            plan("#[redacted(mask, skip)] password: String").mode,
            Mode::Skip,
            "when `mask` and `skip` are both given, the later one wins"
        );
    }

    #[test]
    fn a_other_attributes_are_ignored() {
        assert_eq!(
            plan("/// The password.\n#[serde(skip)] #[allow(dead_code)] password: String"),
            Plan {
                label: "password".into(),
                mode: Mode::Show
            },
            "only attributes named `redacted` belong to this derive"
        );
    }

    #[test]
    fn a_unknown_arguments_are_errors() {
        for source in [
            "#[redacted(hide)] password: String",
            "#[redacted] password: String",
            "#[redacted(rename)] password: String",
            "#[redacted(rename = 7)] password: String",
            "#[redacted = \"mask\"] password: String",
        ] {
            assert!(
                plan_field(&field(source)).is_err(),
                "`{source}` must be rejected with an Err, not accepted and not a panic"
            );
        }
    }

    /// The types of the fields `title` and `rows` of VIEW, with their spans.
    fn title_and_rows(view: &DeriveInput) -> Vec<&Type> {
        let Data::Struct(data) = &view.data else {
            unreachable!("VIEW is a struct")
        };
        let types: Vec<&Type> = data.fields.iter().map(|field| &field.ty).collect();
        vec![types[0], types[2]]
    }

    #[test]
    fn b_predicates_are_added_after_the_existing_ones() {
        let view = input(VIEW);
        let clause =
            where_with_debug(&view.generics, &title_and_rows(&view)).expect("there are predicates");
        let predicates: Vec<String> = clause.predicates.iter().map(compact).collect();
        assert_eq!(
            predicates,
            [
                "T:Send",
                "&'astr:::core::fmt::Debug",
                "[u8;N]:::core::fmt::Debug"
            ],
            "first the struct's own predicates, then one `TYPE: ::core::fmt::Debug` per shown type"
        );
    }

    #[test]
    fn b_a_new_bound_points_at_the_field_s_type() {
        let view = input(VIEW);
        let clause =
            where_with_debug(&view.generics, &title_and_rows(&view)).expect("there are predicates");
        for (predicate, ty) in clause.predicates.iter().skip(1).zip(["&'a str", "[u8; N]"]) {
            let expected = at(VIEW, ty);
            let bound: Vec<_> = flatten(quote!(#predicate))
                .into_iter()
                .filter(|token| {
                    matches!(token, TokenTree::Ident(ident)
                        if ident == "core" || ident == "fmt" || ident == "Debug")
                })
                .collect();
            assert_eq!(bound.len(), 3, "the bound is `::core::fmt::Debug`");
            for token in bound {
                let start = token.span().start();
                assert_eq!(
                    (start.line, start.column),
                    expected,
                    "the token `{token}` of the new bound must have the span of the field type \
                     `{ty}`. With the default span, the compiler reports a type without `Debug` \
                     at the derive attribute, and the user has to search for the field"
                );
            }
        }
    }

    #[test]
    fn b_no_predicates_means_no_where_clause() {
        let plain = input("struct Plain { a: u8 }");
        assert!(
            where_with_debug(&plain.generics, &[]).is_none(),
            "without any predicate there must be no `where` keyword at all"
        );
        let ty: Type = syn::parse_quote!(u8);
        let clause = where_with_debug(&plain.generics, &[&ty]).expect("one predicate");
        assert_eq!(compact(&clause), "whereu8:::core::fmt::Debug");
    }

    #[test]
    fn c_the_impl_header_carries_the_generics() {
        let output = redacted_impl(&input(VIEW)).expect("VIEW is a valid input");
        let item: ItemImpl = parse(output, "one `impl` block");
        let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
        assert_eq!(compact(path), "::m5_proc_macros::Redacted");
        assert_eq!(
            compact(&item.generics.params),
            "'a,T:Clone,constN:usize",
            "after `impl`: every parameter with its bounds, and without its default"
        );
        assert_eq!(
            compact(&item.self_ty),
            "View<'a,T,N>",
            "after the type's name: only the names of the parameters"
        );
    }

    #[test]
    fn c_only_shown_fields_add_bounds() {
        let output = redacted_impl(&input(VIEW)).expect("VIEW is a valid input");
        let item: ItemImpl = parse(output, "one `impl` block");
        let clause = item.generics.where_clause.as_ref().expect("a where clause");
        let predicates: Vec<String> = clause.predicates.iter().map(compact).collect();
        assert_eq!(
            predicates,
            [
                "T:Send",
                "&'astr:::core::fmt::Debug",
                "[u8;N]:::core::fmt::Debug"
            ],
            "the struct's own predicate, then one bound per shown field type. The masked \
             field `secret: T` and the skipped field `cache: Vec<T>` are never printed, so \
             they must not add a bound"
        );
    }

    #[test]
    fn c_a_plain_struct_gets_a_plain_impl() {
        let output = redacted_impl(&input(
            "struct Hidden { #[redacted(mask)] a: u8, #[redacted(skip)] b: u8 }",
        ))
        .expect("a valid input");
        let item: ItemImpl = parse(output.clone(), "one `impl` block");
        assert!(item.generics.params.is_empty());
        assert!(
            item.generics.where_clause.is_none(),
            "no shown field and no where clause on the struct: the impl has no where clause"
        );
        assert_absolute(
            &output,
            &["String", "std", "core", "alloc", "format", "m5_proc_macros"],
        );
    }

    #[test]
    fn c_other_shapes_and_bad_attributes_are_errors() {
        for source in [
            "struct Pair(u8, u8);",
            "struct Unit;",
            "enum Level { Low, High }",
            "struct Login { #[redacted(hide)] password: String }",
        ] {
            assert!(
                redacted_impl(&input(source)).is_err(),
                "`{source}` must give an Err"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `compact` prints tokens and removes all white space.
    #[test]
    fn p_predictions() {
        let view = input(VIEW);
        let (impl_generics, ty_generics, _) = view.generics.split_for_impl();

        // P1. The piece that goes after the type's name.
        let p1: &str = guess();
        assert_eq!(
            p1,
            compact(&ty_generics),
            "P1: ty_generics of VIEW (left: your prediction, right: actual)"
        );

        // P2. The piece that goes after `impl`.
        let p2: &str = guess();
        assert_eq!(
            p2,
            compact(&impl_generics),
            "P2: impl_generics of VIEW (left: your prediction, right: actual)"
        );

        // P3. The same header for a struct without any generics.
        let plain = input("struct Plain { a: u8 }");
        let (impl_generics, ty_generics, where_clause) = plain.generics.split_for_impl();
        let p3: &str = guess();
        assert_eq!(
            p3,
            compact(quote!(impl #impl_generics Trait for Plain #ty_generics #where_clause {})),
            "P3: the impl header of a struct without generics (left: your prediction, right: actual)"
        );

        // P4. How many attributes does this field have for a macro?
        let documented = field("/// Shown to nobody.\n#[redacted(mask)] #[serde(skip)] key: u8");
        let p4: usize = guess();
        assert_eq!(
            p4,
            documented.attrs.len(),
            "P4: attributes on the field, doc comment included (left: your prediction, right: actual)"
        );
    }
}
