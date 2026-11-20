//! CODE REVIEW 01 — A teammate's derive
//! Run: cargo test c01_
//! Then: cargo clippy --all-targets
//!
//! A teammate wrote `#[derive(Reset)]` in a hurry and tried it on one struct.
//! It works there. Your job:
//! 1. Before running anything, read the code and write a review in NOTES.md:
//!    list every input for which the macro panics, generates code that does
//!    not compile, or silently does the wrong thing, and every habit that
//!    makes its error messages worse than they could be.
//! 2. Run the tests. Compare what they found with your review.
//! 3. Fix it. Keep the name and the signature of `reset_impl`. Everything
//!    else is yours to restructure.
//! 4. Run clippy and resolve its warnings for this file.
//!
//! # Contract
//!
//! `#[derive(Reset)]` implements `::m5_proc_macros::Reset` for a struct with
//! named fields, a tuple struct or a unit struct, with or without generic
//! parameters. `reset(&mut self)` assigns `Default::default()` to every field,
//! in declaration order, except to fields marked `#[reset(keep)]`.
//!
//! Generic parameters and the where clause of the struct are carried over.
//! The derive adds no bounds: a field type without `Default` is an error in
//! the generated code. The compiler must report it at that field's type, not
//! at the `#[derive(...)]` line (`tests/ui/c01/fail_no_default.stderr`).
//!
//! Errors, as `compile_error!` calls and nothing else, all in one run:
//! - an enum or a union: ``` `Reset` can only be derived for structs ``` spanned
//!   on the keyword `enum` or `union`;
//! - any argument of `#[reset(...)]` other than `keep`:
//!   ``unknown `reset` argument: expected `keep` `` spanned on the argument;
//! - `#[reset]` without parentheses, and input that is not a struct, enum or
//!   union: pass on syn's error.
//!
//! The macro must never panic.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Field};

/// `#[derive(Reset)]`, from the raw input tokens to the output tokens.
pub(crate) fn reset_impl(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse2(input).unwrap();
    let name = input.ident.clone();
    let fields: Vec<Field> = match input.data.clone() {
        Data::Struct(data) => data.fields.iter().cloned().collect(),
        _ => panic!("Reset only works on structs"),
    };

    let mut body = String::new();
    for i in 0..fields.len() {
        let field = &fields[i];
        if is_kept(&field.attrs) {
            continue;
        }
        match &field.ident {
            Some(ident) => {
                body += &format!("self.{} = Default::default();", ident);
            }
            None => {
                let index = i;
                body += &quote!(self.#index = Default::default();).to_string();
            }
        }
    }
    let body: TokenStream = body.parse().unwrap();

    quote! {
        impl m5_proc_macros::Reset for #name {
            fn reset(&mut self) {
                #body
            }
        }
    }
}

/// True when the field is marked `#[reset(keep)]`.
fn is_kept(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs.iter() {
        if attr.path().is_ident("reset") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{
        assert_absolute, at, compact, errors_in, flatten, is_clean, parse, predicates, tokens,
    };
    use proc_macro2::TokenTree;
    use syn::ItemImpl;

    const COUNTER: &str = "
/// Counts things.
#[derive(Debug)]
pub struct Counter {
    count: u32,
    #[reset(keep)]
    label: String,
    #[doc(hidden)]
    history: Vec<u32>,
}";

    fn expand(source: &str) -> TokenStream {
        std::panic::catch_unwind(|| reset_impl(tokens(source))).unwrap_or_else(|_| {
            panic!(
                "reset_impl panicked on:\n{source}\nA macro must report every problem as a \
                 `compile_error!` with a span. The user of a macro that panics sees only \
                 \"proc-macro derive panicked\" at the derive attribute"
            )
        })
    }

    #[test]
    fn a_named_fields_are_reset_except_the_kept_ones() {
        let output = expand(COUNTER);
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let item: ItemImpl = parse(output, "one impl block");
        assert_eq!(compact(&item.self_ty), "Counter");
        let body = compact(&item);
        assert!(
            body.contains("self.count="),
            "`count` must be reset; output was: {body}"
        );
        assert!(
            body.contains("self.history="),
            "`history` must be reset; output was: {body}"
        );
        assert!(
            !body.contains("self.label"),
            "`label` is marked `#[reset(keep)]` and must not be touched; output was: {body}"
        );
        assert!(
            body.find("self.count=") < body.find("self.history="),
            "fields are reset in declaration order"
        );
    }

    #[test]
    fn a_generic_parameters_and_the_where_clause_are_carried_over() {
        let output = expand(
            "struct Window<'a, T: Clone = u8, const N: usize = 4> where T: Default {
                 first: T,
                 grid: [u8; N],
                 #[reset(keep)] title: &'a str,
             }",
        );
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let item: ItemImpl = parse(output, "one impl block");
        assert_eq!(
            compact(&item.generics.params),
            "'a,T:Clone,constN:usize",
            "after `impl`: every parameter with its bounds, without defaults"
        );
        assert_eq!(compact(&item.self_ty), "Window<'a,T,N>");
        assert_eq!(predicates(&item.generics), ["T:Default"]);
    }

    #[test]
    fn a_tuple_and_unit_structs_work() {
        let output = expand("struct Pair(u8, #[reset(keep)] String, Vec<u8>);");
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let item: ItemImpl = parse(output.clone(), "one impl block");
        let body = compact(&item);
        assert!(
            body.contains("self.0=") && body.contains("self.2=") && !body.contains("self.1"),
            "a tuple struct's fields are `self.0`, `self.1`, ...; output was: {body}"
        );
        for token in flatten(output) {
            if let TokenTree::Literal(literal) = &token {
                let text = literal.to_string();
                assert!(
                    !text.ends_with("usize"),
                    "the output contains the literal `{text}`. `self.0usize` is not a field: \
                     the compiler rejects a suffix on a tuple index"
                );
            }
        }
        let unit = expand("struct Unit;");
        assert!(
            is_clean(&unit),
            "a unit struct has nothing to reset, and that is valid"
        );
        let _: ItemImpl = parse(unit, "one impl block");
    }

    #[test]
    fn b_the_field_tokens_keep_their_spans() {
        let output = expand(COUNTER);
        for field in ["count", "history"] {
            let expected = at(COUNTER, &format!("{field}:"));
            let reused = flatten(output.clone()).into_iter().any(|token| {
                let start = token.span().start();
                matches!(&token, TokenTree::Ident(ident) if ident == field)
                    && (start.line, start.column) == expected
            });
            assert!(
                reused,
                "no `{field}` token in the output has the span of the field in the input. \
                 Code that is printed to a string and parsed again has lost every span, so \
                 an error about a field without `Default` would point at the derive attribute"
            );
        }
    }

    #[test]
    fn b_library_names_are_written_as_absolute_paths() {
        let output = expand(COUNTER);
        assert_absolute(
            &output,
            &["Default", "Reset", "m5_proc_macros", "core", "std"],
        );
        let item: ItemImpl = parse(output, "one impl block");
        let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
        assert_eq!(compact(path), "::m5_proc_macros::Reset");
    }

    #[test]
    fn c_enums_and_unions_are_errors_at_the_keyword() {
        for (source, keyword) in [
            ("\npub enum Level { Low, High }", "enum"),
            ("\nunion Bits { int: u32, float: f32 }", "union"),
        ] {
            let output = expand(source);
            assert_eq!(
                errors_in(output.clone()),
                [(
                    "`Reset` can only be derived for structs".to_owned(),
                    at(source, keyword)
                )]
            );
            assert!(
                !compact(&output).contains("impl"),
                "on an error the output is the `compile_error!` call only"
            );
        }
    }

    #[test]
    fn c_a_misspelled_argument_is_an_error_not_a_keep() {
        let source = "
struct Counter {
    #[reset(kep)]
    count: u32,
    #[reset(keep, skip)]
    label: String,
}";
        const UNKNOWN: &str = "unknown `reset` argument: expected `keep`";
        assert_eq!(
            errors_in(expand(source)),
            [
                (UNKNOWN.to_owned(), at(source, "kep")),
                (UNKNOWN.to_owned(), at(source, "skip")),
            ],
            "`#[reset(kep)]` is a typing mistake. Treating every `#[reset(..)]` as `keep` \
             hides it, and the field silently keeps its value"
        );
    }

    #[test]
    fn c_strange_input_is_an_error_and_never_a_panic() {
        for source in ["", "fn f() {}", "struct S { #[reset] a: u8 }"] {
            assert!(
                !errors_in(expand(source)).is_empty(),
                "`{source}` must be an error"
            );
        }
    }
}
