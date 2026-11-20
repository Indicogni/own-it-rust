//! Support code for the unit tests. You do not need to edit this file.
//!
//! - [`guess`] marks a prediction you have not written yet.
//! - [`compact`] turns tokens into text without white space, so that a test
//!   does not depend on how tokens are printed.
//! - [`predicates`] lists the predicates of a where clause as compact text.
//! - [`parse`] turns a macro's output back into a syntax tree, and says what
//!   was wrong when that fails.
//! - [`errors_in`] lists the `compile_error!` calls in a macro's output, with
//!   the line and column that each one points at.

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;

/// Placeholder for a prediction. Replace the call with your answer before
/// running the lesson, for example `let p1: usize = 5;`.
#[track_caller]
pub(crate) fn guess<T>() -> T {
    panic!("PREDICT: replace `guess()` with your written prediction, then run again")
}

/// The text of `tokens` with every white-space character removed.
///
/// `compact(quote!(Vec < u8 >))` is `"Vec<u8>"`. Tokens have no white space;
/// printing puts some between them, and tests should not depend on where.
pub(crate) fn compact(tokens: impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .split_whitespace()
        .collect()
}

/// The predicates of a where clause, each as compact text. Empty when there is
/// no where clause. A comma after the last predicate makes no difference.
pub(crate) fn predicates(generics: &syn::Generics) -> Vec<String> {
    generics
        .where_clause
        .iter()
        .flat_map(|clause| clause.predicates.iter().map(compact))
        .collect()
}

/// Every token of `tokens` in order, including the tokens inside groups. A
/// group itself is not listed, only its contents.
pub(crate) fn flatten(tokens: TokenStream) -> Vec<TokenTree> {
    let mut flat = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Group(group) => flat.extend(flatten(group.stream())),
            other => flat.push(other),
        }
    }
    flat
}

/// Parses `tokens` as a `T`, or fails the test and shows the tokens.
#[track_caller]
pub(crate) fn parse<T: syn::parse::Parse>(tokens: TokenStream, what: &str) -> T {
    match syn::parse2::<T>(tokens.clone()) {
        Ok(tree) => tree,
        Err(error) => panic!(
            "the macro's output should be {what}, but it does not parse: {error}\n\
             output was: {tokens}"
        ),
    }
}

/// Parses source text into tokens. Spans then carry the line (counted from 1)
/// and the column (counted from 0) of each token in `source`.
#[track_caller]
pub(crate) fn tokens(source: &str) -> TokenStream {
    source
        .parse()
        .unwrap_or_else(|error| panic!("test input is not valid Rust tokens: {error}"))
}

/// A position in a test's source text: line counted from 1, column from 0.
/// This is how compilers and editors count, and how `Span::start` counts.
pub(crate) type Position = (usize, usize);

/// The position at which `needle` first appears in `source`.
#[track_caller]
pub(crate) fn at(source: &str, needle: &str) -> Position {
    let offset = source
        .find(needle)
        .unwrap_or_else(|| panic!("the test source does not contain {needle:?}"));
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |text| text.chars().count());
    (line, column)
}

/// The `compile_error!` calls in a macro's output, in order: each message with
/// the position that its span starts at.
///
/// A span that does not come from the test's source (for example
/// `Span::call_site()`) reports the position `(1, 0)`.
pub(crate) fn errors_in(output: TokenStream) -> Vec<(String, Position)> {
    let mut found = Vec::new();
    let mut tokens = output.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Ident(ident) if ident == "compile_error" => {
                let start = ident.span().start();
                // `!` and then the group that holds the message.
                let bang = tokens.next();
                let group = tokens.next();
                if let (Some(TokenTree::Punct(bang)), Some(TokenTree::Group(group))) = (bang, group)
                    && bang.as_char() == '!'
                    && let Ok(message) = syn::parse2::<syn::LitStr>(group.stream())
                {
                    found.push((message.value(), (start.line, start.column)));
                }
            }
            TokenTree::Group(group) if group.delimiter() != Delimiter::None => {
                found.extend(errors_in(group.stream()));
            }
            _ => {}
        }
    }
    found
}

/// True when the macro's output contains no `compile_error!` call.
pub(crate) fn is_clean(output: &TokenStream) -> bool {
    errors_in(output.clone()).is_empty()
}

/// Fails the test when an identifier from `names` appears in `output` without
/// `::` in front of it. Generated code must name library items by absolute
/// paths such as `::core::option::Option`, because the user's module may have
/// its own `Option`.
#[track_caller]
pub(crate) fn assert_absolute(output: &TokenStream, names: &[&str]) {
    let flat = flatten(output.clone());
    for (index, token) in flat.iter().enumerate() {
        let TokenTree::Ident(ident) = token else {
            continue;
        };
        if !names.iter().any(|name| ident == name) {
            continue;
        }
        let colon = |token: Option<&TokenTree>| matches!(token, Some(TokenTree::Punct(p)) if p.as_char() == ':');
        let qualified = index >= 2 && colon(flat.get(index - 1)) && colon(flat.get(index - 2));
        assert!(
            qualified,
            "the generated code uses `{ident}` without a path in front of it. Write the full \
             path, starting with `::`, so that the code means the same thing in every module \
             it is expanded in.\noutput was: {output}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn compact_removes_white_space() {
        assert_eq!(compact(quote!(Vec<u8>)), "Vec<u8>");
        assert_eq!(compact(quote!(a.b())), "a.b()");
    }

    #[test]
    fn at_counts_lines_from_one_and_columns_from_zero() {
        let source = "enum E {\n    A,\n    B(u8),\n}";
        assert_eq!(at(source, "enum"), (1, 0));
        assert_eq!(at(source, "B"), (3, 4));
        assert_eq!(at(source, "(u8)"), (3, 5));
    }

    #[test]
    fn errors_in_reads_messages_and_positions() {
        let source = "\nstruct Job;";
        let item: syn::DeriveInput = syn::parse2(tokens(source)).unwrap();
        let error = syn::Error::new_spanned(&item.ident, "bad name");
        assert_eq!(
            errors_in(error.to_compile_error()),
            [("bad name".to_owned(), at(source, "Job"))]
        );
        assert!(is_clean(&quote!(
            struct Fine;
        )));
    }

    #[test]
    #[should_panic(expected = "without a path in front of it")]
    fn assert_absolute_rejects_a_bare_name() {
        assert_absolute(
            &quote!(
                fn f() -> Option<u8> {
                    None
                }
            ),
            &["Option"],
        );
    }

    #[test]
    fn assert_absolute_accepts_a_full_path() {
        assert_absolute(
            &quote!(
                fn f() -> ::core::option::Option<u8> {
                    ::core::option::Option::None
                }
            ),
            &["Option", "None", "core"],
        );
    }

    #[test]
    #[should_panic(expected = "PREDICT")]
    fn guess_panics_until_replaced() {
        let _: u8 = guess();
    }
}
