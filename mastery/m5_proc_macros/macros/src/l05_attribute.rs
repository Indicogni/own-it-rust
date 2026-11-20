//! LESSON 05 — An attribute macro that keeps the item intact
//! Run: cargo test l05_
//! Read first: "Lesson 05" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. A derive adds code next to an item. What does an attribute macro do with
//!    the item it is written on?
//! 2. A function can end at its last line, at a `return`, at a `?`, or with a
//!    panic. Which code runs in all four cases?
//!
//! IMPLEMENT the tasks below. From this lesson on you get fewer signatures:
//! `LoggedArgs` and `logged` are fixed, every helper is yours to design.
//! Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Which two token streams does an attribute macro receive,
//! and what happens to the original item? Why is "parse the function, change
//! one field, print it again" safer than building a new function from parts?
//! Why does the macro return the item even when it reports an error?
//!
//! COMPILER / TRANSFER LAB (optional): bind the guard with `let _ = ...`
//! instead of a named variable and run `cargo test l05_`. Explain the order of
//! the lines in the log. Then look at `tests/ui/l05/fail_args.stderr`: after
//! the three errors of the macro, the compiler reports a type error in `main`.
//! Return only the `compile_error!` (without the item), run the test again,
//! and see which error the compiler can no longer find.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, LitStr, Token};

/// What the user wrote between the parentheses of `#[logged(...)]`.
/// `#[logged]` without parentheses gives an empty token stream.
pub(crate) struct LoggedArgs {
    /// `label = "text"`, when given.
    pub label: Option<LitStr>,
}

/// A. Parse the arguments of the attribute.
///
/// - Nothing: no label.
/// - `label = "text"`, with or without a comma after it: that literal.
///
/// Errors:
/// - a name other than `label`: ``unknown argument `NAME`: expected `label = "..."` ``
///   spanned on the name;
/// - `label` a second time: ``duplicate argument `label` `` spanned on the
///   second `label`;
/// - anything else that does not fit (`label` without `=`, a value that is not
///   a string literal, two arguments without a comma): pass on syn's error.
impl Parse for LoggedArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        todo!("Implement this task")
    }
}

/// B. The attribute macro `#[logged]`. `args` holds the tokens between the
/// parentheses of the attribute, `item` holds the item under the attribute.
/// The returned tokens *replace* the item. This function must never panic.
///
/// When `item` is a function with a body, the output is that same function
/// with exactly one statement added in front of its other statements: a `let`
/// that binds the value of `::m5_proc_macros::calls::enter(LABEL)` to a named
/// variable. The guard must live until the function ends, so the pattern must
/// not be `_`. Choose a name that a user's function will not use for a
/// parameter or a variable of its own, such as `__logged_guard`. LABEL is a
/// string literal: the `label` argument, or else the function's name without
/// `r#`.
///
/// Nothing else changes: attributes, visibility, `const`/`async`/`unsafe`,
/// name, generic parameters, parameters, return type, where clause and all
/// the statements of the body stay as they are, in order.
///
/// Errors:
/// - task A reports one;
/// - `item` is not a function with a body: pass on syn's error from parsing
///   an `ItemFn`;
/// - `item` is a `const fn`, which cannot write to a log at compile time:
///   ``` `#[logged]` cannot be used on a `const fn` ``` spanned on the keyword
///   `const`.
///
/// On an error, the output is the error's `compile_error!` call followed by
/// the tokens of `item`, unchanged. The user's item must not disappear
/// because an argument was wrong: with the item in place, the compiler goes
/// on to check the code that uses it, and reports those mistakes in the same
/// run.
pub(crate) fn logged(args: TokenStream, item: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{at, compact, errors_in, guess, is_clean, parse, tokens};
    use syn::{Item, Pat, Stmt};

    const LOAD: &str = "
/// Loads one value.
#[inline]
#[must_use]
pub(crate) async unsafe fn load<'a, T: Clone>(
    &'a mut self,
    key: &str,
    mut retries: u8,
) -> Option<T>
where
    T: Send,
{
    if retries == 0 {
        return None;
    }
    retries -= 1;
    self.fetch(key, retries).await
}";

    fn args(source: &str) -> syn::Result<LoggedArgs> {
        syn::parse2(tokens(source))
    }

    fn problem(source: &str) -> (String, (usize, usize)) {
        let error = args(source)
            .err()
            .unwrap_or_else(|| panic!("`#[logged({source})]` must be rejected"));
        let start = error.span().start();
        (error.to_string(), (start.line, start.column))
    }

    #[test]
    fn a_no_arguments_means_no_label() {
        assert!(args("").expect("`#[logged]` is valid").label.is_none());
    }

    #[test]
    fn a_label_is_read() {
        let source = "\n label = \"db.load\"";
        let label = args(source)
            .expect("valid")
            .label
            .expect("a label was given");
        assert_eq!(label.value(), "db.load");
        let start = label.span().start();
        assert_eq!(
            (start.line, start.column),
            at(source, "\"db.load\""),
            "keep the user's literal token"
        );
        assert!(
            args("label = \"x\",").is_ok(),
            "a comma after the argument is allowed"
        );
    }

    #[test]
    fn a_unknown_and_duplicate_arguments_are_rejected_at_the_name() {
        let source = "\n level = 3";
        assert_eq!(
            problem(source),
            (
                "unknown argument `level`: expected `label = \"...\"`".to_owned(),
                at(source, "level")
            )
        );
        let source = "\n label = \"a\", label = \"b\"";
        assert_eq!(
            problem(source),
            (
                "duplicate argument `label`".to_owned(),
                at(source, "label = \"b\"")
            )
        );
    }

    #[test]
    fn a_other_mistakes_keep_syn_s_error() {
        let source = "\n label = 7";
        assert_eq!(
            problem(source),
            ("expected string literal".to_owned(), at(source, "7"))
        );
        for source in [
            "label",
            "label \"a\"",
            "\"a\"",
            "label = \"a\" label = \"b\"",
        ] {
            assert!(
                args(source).is_err(),
                "`#[logged({source})]` must be rejected"
            );
        }
    }

    #[test]
    fn b_everything_but_the_body_is_unchanged() {
        let before: ItemFn = parse(tokens(LOAD), "a function");
        let output = logged(TokenStream::new(), tokens(LOAD));
        assert!(
            is_clean(&output),
            "a valid function must expand without errors: {output}"
        );
        let after: ItemFn = parse(output, "one function");
        assert_eq!(
            after.attrs, before.attrs,
            "attributes, doc comments included, must be kept in order"
        );
        assert_eq!(after.vis, before.vis, "the visibility must be kept");
        assert_eq!(
            compact(&after.sig),
            compact(&before.sig),
            "the signature must be kept: async, unsafe, generics, parameters, return type, \
             where clause"
        );
        assert_eq!(after.sig, before.sig);
        assert_eq!(
            after.block.stmts.len(),
            before.block.stmts.len() + 1,
            "exactly one statement is added"
        );
        assert_eq!(
            after.block.stmts[1..],
            before.block.stmts[..],
            "the function's own statements must follow unchanged, in order"
        );
    }

    #[test]
    fn b_the_first_statement_binds_the_guard_to_a_name() {
        let after: ItemFn = parse(logged(TokenStream::new(), tokens(LOAD)), "one function");
        let Some(Stmt::Local(local)) = after.block.stmts.first() else {
            panic!("the first statement of the body must be a `let`");
        };
        assert!(
            !matches!(local.pat, Pat::Wild(_)),
            "`let _ = enter(..)` drops the guard at once, so the exit line would be written \
             before the body runs. Bind the guard to a variable"
        );
        let init = local.init.as_ref().expect("the `let` has a value");
        assert_eq!(
            compact(&init.expr),
            "::m5_proc_macros::calls::enter(\"load\")",
            "without a `label` argument, the label is the function's name"
        );
    }

    #[test]
    fn b_the_label_argument_replaces_the_name() {
        let output = logged(tokens("label = \"db.load\""), tokens(LOAD));
        assert!(
            compact(&output).contains("::m5_proc_macros::calls::enter(\"db.load\")"),
            "output was: {output}"
        );
        let raw = logged(TokenStream::new(), tokens("fn r#match() {}"));
        assert!(
            compact(&raw).contains("enter(\"match\")"),
            "the default label drops `r#`; output was: {raw}"
        );
    }

    #[test]
    fn b_an_argument_error_keeps_the_function() {
        let source = "\n level = 3";
        let output = logged(tokens(source), tokens(LOAD));
        assert_eq!(
            errors_in(output.clone()),
            [(
                "unknown argument `level`: expected `label = \"...\"`".to_owned(),
                at(source, "level")
            )]
        );
        let file: syn::File = parse(output, "a compile_error! call and the function");
        let before: ItemFn = parse(tokens(LOAD), "a function");
        assert!(
            file.items
                .iter()
                .any(|item| matches!(item, Item::Fn(function) if *function == before)),
            "when the arguments are wrong, the output must still hold the user's function, \
             unchanged. Without it, the compiler cannot check any code that calls `load` \
             until the argument is fixed"
        );
    }

    #[test]
    fn b_an_item_that_is_no_function_is_an_error_and_is_kept() {
        for source in ["struct Job { id: u32 }", "fn declared_only(&self);"] {
            let output = std::panic::catch_unwind(|| logged(TokenStream::new(), tokens(source)))
                .unwrap_or_else(|_| {
                    panic!("logged panicked on `{source}`: return a compile_error! instead")
                });
            assert_eq!(
                errors_in(output.clone()).len(),
                1,
                "`{source}` is not a function with a body: pass on syn's error"
            );
            assert!(
                compact(&output).ends_with(&compact(tokens(source))),
                "after the compile_error!, the output must hold the item's tokens unchanged; \
                 output was: {output}"
            );
        }
    }

    #[test]
    fn b_a_const_fn_is_rejected_at_the_keyword_and_kept() {
        let source = "\npub const fn limit() -> u32 { 100 }";
        let output = logged(TokenStream::new(), tokens(source));
        assert_eq!(
            errors_in(output.clone()),
            [(
                "`#[logged]` cannot be used on a `const fn`".to_owned(),
                at(source, "const")
            )],
            "a const fn cannot call `enter`. Say so yourself, at the keyword: the compiler's \
             own two errors about the generated code would point at the attribute"
        );
        assert!(
            compact(&output).ends_with(&compact(tokens(source))),
            "after the compile_error!, the output must hold the function unchanged"
        );
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `compact` prints tokens and removes all white space.
    #[test]
    fn p_predictions() {
        // P1. How many attributes does syn find on this function?
        let function: ItemFn = syn::parse_quote! {
            #[inline]
            /** Adds one. */
            pub fn add_one(n: u8) -> u8 {
                let next = n + 1;
                record(next);
                next
            }
        };
        let p1: usize = guess();
        assert_eq!(
            p1,
            function.attrs.len(),
            "P1: attributes on the function (left: your prediction, right: actual)"
        );

        // P2. How many statements does the body have, and is the last one an
        // expression without a semicolon?
        let last_is_tail = matches!(function.block.stmts.last(), Some(Stmt::Expr(_, None)));
        let p2: (usize, bool) = guess();
        assert_eq!(
            p2,
            (function.block.stmts.len(), last_is_tail),
            "P2: (statements, last one is a tail expression) (left: your prediction, right: actual)"
        );

        // P3. Which tokens belong to the signature `function.sig`?
        let function: ItemFn = syn::parse_quote! {
            pub(crate) async fn load<T>(&self, key: T) -> u8 where T: Copy { 0 }
        };
        let p3: &str = guess();
        assert_eq!(
            p3,
            compact(&function.sig),
            "P3: the signature's tokens (left: your prediction, right: actual)"
        );
    }
}
