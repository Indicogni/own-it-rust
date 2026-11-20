//! LESSON 07 — Hygiene, crate paths, and how to test a macro
//! Run: cargo test l07_
//! Read first: "Lesson 07" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. Your macro writes `let tries = 0;` in front of the user's expression,
//!    and the user's expression reads the user's own variable `tries`. Which
//!    `tries` does the expression see?
//! 2. A user renames your crate in `Cargo.toml`. Which token in the generated
//!    code stops working?
//!
//! IMPLEMENT the contract below. Only the three entry points are given. Keep
//! the supplied tests and add one edge case.
//! EXPLAIN afterward: Which names in generated code are resolved where the
//! macro is *used*, and what are the two ways to protect them (one for local
//! variables, one for paths)? Why can a unit test on tokens not prove that a
//! macro is hygienic, and which kind of test can?
//!
//! COMPILER / TRANSFER LAB (optional): give the two local variables of
//! `attempt!` the span `Span::call_site()` and run `cargo test l07_`. Read the
//! compiler's message for `tests/ui/l07/pass_attempt_hygiene.rs`. Then expand
//! one use of the macro (your editor's expansion view, or `cargo expand` when
//! it is installed) and notice that the printed code looks the same in both
//! versions: hygiene is in the spans, not in the text.
//!
//! # Contract
//!
//! ## A. `attempt!(MAX, EXPR)`
//!
//! MAX is an expression of type `u32`, EXPR an expression of type
//! `Result<T, E>`. A comma after EXPR is allowed. The macro expands to one
//! expression of type `Result<T, E>`:
//!
//! - MAX is evaluated exactly once, before the first try;
//! - EXPR is evaluated again and again until it gives `Ok`, at most MAX times
//!   and at least once (also when MAX is 0);
//! - the value is the first `Ok`, or else the last `Err`.
//!
//! EXPR is written once in the expansion, in the place where it runs: not
//! inside a closure and not inside a new function. So `?` and `return` in
//! EXPR act on the function that uses the macro.
//!
//! Hygiene, checked by the compiler in `tests/ui/l07/`:
//! 1. The expansion keeps MAX in a local variable named `__attempt_max`. Every
//!    other variable that it declares, in a `let` or in a `match` pattern, has
//!    a name that starts with `__attempt_`.
//! 2. MAX and EXPR never see a variable of the expansion, and they still see
//!    every variable of the user, also when the user has variables with
//!    exactly these names.
//! 3. The expansion means the same when the user's module has its own items
//!    called `Result`, `Ok`, `Err`, `Option`, `Some`, `None` or `u32`, and when
//!    it has constants or unit structs with names such as `max` or `tries`.
//!
//! Input of another shape: pass on syn's error as a `compile_error!`.
//!
//! ## B. `#[derive(Named)]`
//!
//! For a struct, an enum or a union, with or without generic parameters:
//!
//! ```text
//! impl<'a, T: Clone> ::m5_proc_macros::Named for Pair<'a, T> where T: Send {
//!     const NAME: &'static str = "Pair";
//! }
//! ```
//!
//! NAME is the type's name without `r#` and without generic arguments.
//!
//! The helper attribute `#[named(crate = PATH)]` on the item replaces
//! `::m5_proc_macros` by PATH, which may be any path, for example `::kit` or
//! `crate::deps::kit`. Then no token of the output may name `m5_proc_macros`.
//!
//! Errors, spanned on the argument's name, all reported together:
//! - an argument other than `crate`: ``unknown `named` argument: expected `crate = path` ``;
//! - `crate` a second time: ``duplicate `crate` argument``.
//!
//! A value that is not a path, and input that is not a struct, enum or union:
//! pass on syn's error. On an error the output is `compile_error!` calls only.
//!
//! ## C. `review_named`
//!
//! You are the reviewer of somebody else's `#[derive(Named)]`. Write the
//! checks that you would put into its test suite. `expand` is the
//! implementation under review, with the signature of `named_impl`.
//! Return `Ok(())` when it follows contract B on the inputs you try, and
//! `Err(text)` with a sentence about the first difference otherwise.
//!
//! The tests run your review against your own `named_impl`, which it must
//! accept, and against several broken implementations, each of which it must
//! reject. Read them in the test module below, after you have thought about
//! what can go wrong. Compare parsed syntax trees or `compact` text, never the
//! raw output of `to_string()`.
//!
//! None of the three functions may panic on any input.

use proc_macro2::TokenStream;

/// A. The function-like macro `attempt!`. See the contract above.
pub(crate) fn attempt(input: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

/// B. `#[derive(Named)]`. See the contract above.
pub(crate) fn named_impl(input: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

/// C. Review an implementation of `#[derive(Named)]`. See the contract above.
pub(crate) fn review_named(expand: &dyn Fn(TokenStream) -> TokenStream) -> Result<(), String> {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{
        assert_absolute, at, compact, errors_in, flatten, guess, is_clean, parse, predicates,
        tokens,
    };
    use proc_macro2::{Span, TokenTree};
    use quote::quote;
    use syn::{Expr, Ident, ItemImpl, Path};

    fn count_ident(output: &TokenStream, name: &str) -> usize {
        flatten(output.clone())
            .iter()
            .filter(|token| matches!(token, TokenTree::Ident(ident) if ident == name))
            .count()
    }

    #[test]
    fn a_the_expansion_is_one_expression_with_each_argument_once() {
        let output = attempt(quote!(user_max(), user_step(&mut state)));
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let _: Expr = parse(output.clone(), "one expression");
        assert_eq!(
            count_ident(&output, "user_max"),
            1,
            "MAX must be written once in the expansion, so that it is evaluated once"
        );
        assert_eq!(
            count_ident(&output, "user_step"),
            1,
            "EXPR must be written once in the expansion: a loop runs it again, a copy is not needed"
        );
        let trailing = attempt(quote!(3, step(),));
        assert!(is_clean(&trailing), "a comma after EXPR is allowed");
    }

    #[test]
    fn a_the_user_s_expression_is_not_moved_into_a_closure() {
        /// Counts closures and functions in a syntax tree.
        #[derive(Default)]
        struct Wrappers(usize);
        impl<'ast> syn::visit::Visit<'ast> for Wrappers {
            fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
                self.0 += 1;
                syn::visit::visit_expr_closure(self, closure);
            }
            fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
                self.0 += 1;
                syn::visit::visit_item_fn(self, function);
            }
        }

        let output = attempt(quote!(3, step()?));
        let expansion: Expr = parse(output.clone(), "one expression");
        let mut wrappers = Wrappers::default();
        syn::visit::Visit::visit_expr(&mut wrappers, &expansion);
        assert_eq!(
            wrappers.0, 0,
            "the expansion contains a closure or a function. Inside one of those, `?` and \
             `return` in EXPR would leave the closure instead of the user's function; \
             output was: {output}"
        );
    }

    #[test]
    fn a_the_expansion_s_variables_have_reserved_names() {
        /// Collects the names that patterns bind.
        #[derive(Default)]
        struct Bound(Vec<String>);
        impl<'ast> syn::visit::Visit<'ast> for Bound {
            fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
                self.0.push(pattern.ident.to_string());
                syn::visit::visit_pat_ident(self, pattern);
            }
        }

        // The arguments declare no variable, so every pattern is the macro's.
        let output = attempt(quote!(3, step()));
        let expansion: Expr = parse(output.clone(), "one expression");
        let mut bound = Bound::default();
        syn::visit::Visit::visit_expr(&mut bound, &expansion);
        assert!(
            bound.0.iter().any(|name| name == "__attempt_max"),
            "the expansion must keep MAX in a variable named `__attempt_max`; it declares \
             {:?}",
            bound.0
        );
        for name in &bound.0 {
            assert!(
                name.starts_with("__attempt_"),
                "the expansion declares `{name}`. In a pattern, a plain name such as `{name}` \
                 means the user's constant or unit struct of that name, when there is one, \
                 and no variable is made. Start every name with `__attempt_`"
            );
        }
    }

    #[test]
    fn a_library_names_are_written_as_absolute_paths() {
        let output = attempt(quote!(3, step()));
        assert_absolute(
            &output,
            &[
                "Result", "Ok", "Err", "Option", "Some", "None", "u32", "core", "std",
            ],
        );
    }

    #[test]
    fn a_wrong_shapes_are_errors_and_never_panic() {
        for source in ["", "3", "3,", "3 step()", "3, step(), extra", "3, step();"] {
            let output =
                std::panic::catch_unwind(|| attempt(tokens(source))).unwrap_or_else(|_| {
                    panic!("attempt panicked on `{source}`: return a compile_error! instead")
                });
            assert_eq!(
                errors_in(output.clone()).len(),
                1,
                "`attempt!({source})` must be one syn error; output was: {output}"
            );
        }
    }

    const PAIR: &str = "
enum Pair<'a, T: Clone = u8>
where
    T: Send,
{
    Left(&'a T),
    Right(T),
}";

    #[test]
    fn b_the_default_path_is_this_module_s_crate() {
        let output = named_impl(tokens(PAIR));
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let item: ItemImpl = parse(output, "one impl block");
        let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
        assert_eq!(compact(path), "::m5_proc_macros::Named");
        assert_eq!(compact(&item.generics.params), "'a,T:Clone");
        assert_eq!(compact(&item.self_ty), "Pair<'a,T>");
        assert_eq!(predicates(&item.generics), ["T:Send"]);
        assert!(
            compact(&item).contains("constNAME:&'staticstr=\"Pair\";"),
            "NAME is the type's name without generic arguments"
        );
        let raw = named_impl(tokens("struct r#Type;"));
        assert!(
            compact(&raw).contains("=\"Type\";"),
            "NAME drops `r#`; output was: {raw}"
        );
    }

    #[test]
    fn b_the_crate_argument_replaces_the_path() {
        for (argument, expected) in [
            ("::kit", "::kit::Named"),
            ("crate::deps::kit", "crate::deps::kit::Named"),
            ("kit", "kit::Named"),
        ] {
            let source = format!("#[named(crate = {argument})] #[derive(Debug)] struct Job;");
            let output = named_impl(tokens(&source));
            assert!(
                is_clean(&output),
                "`{source}` is valid; output was: {output}"
            );
            let item: ItemImpl = parse(output.clone(), "one impl block");
            let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
            assert_eq!(compact(path), expected);
            assert_eq!(
                count_ident(&output, "m5_proc_macros"),
                0,
                "with `crate = {argument}`, no token of the output may name `m5_proc_macros`: \
                 under that name the crate may not exist for this user"
            );
        }
    }

    #[test]
    fn b_argument_errors_point_at_the_argument() {
        let source = "\n#[named(krate = ::kit, crate = ::a)]\n#[named(crate = ::b)]\nstruct Job;";
        let output = named_impl(tokens(source));
        assert_eq!(
            errors_in(output.clone()),
            [
                (
                    "unknown `named` argument: expected `crate = path`".to_owned(),
                    at(source, "krate")
                ),
                (
                    "duplicate `crate` argument".to_owned(),
                    at(source, "crate = ::b")
                ),
            ],
            "report every wrong argument, each at its own name"
        );
        assert!(
            !compact(&output).contains("impl"),
            "on an error the output is the `compile_error!` calls only"
        );
    }

    #[test]
    fn b_never_panics() {
        for source in [
            "",
            "fn f() {}",
            "#[named] struct S;",
            "#[named(crate = 7)] struct S;",
        ] {
            let output =
                std::panic::catch_unwind(|| named_impl(tokens(source))).unwrap_or_else(|_| {
                    panic!("named_impl panicked on `{source}`: return a compile_error! instead")
                });
            assert!(
                !errors_in(output.clone()).is_empty(),
                "`{source}` must be an error; output was: {output}"
            );
        }
    }

    // ---- C: broken implementations, each made from your own `named_impl` ----

    /// Applies `change` to the impl block that `named_impl` returns. When the
    /// output is not an impl block, because your `named_impl` reports an error
    /// for this input, it is passed on unchanged: a broken implementation
    /// handles wrong input exactly as yours does.
    fn broken(input: TokenStream, change: impl Fn(&mut ItemImpl)) -> TokenStream {
        let output = named_impl(input);
        let Ok(mut item) = syn::parse2::<ItemImpl>(output.clone()) else {
            return output;
        };
        change(&mut item);
        quote!(#item)
    }

    fn trait_path(item: &mut ItemImpl) -> &mut Path {
        &mut item.trait_.as_mut().expect("an impl of a trait").0
    }

    /// Writes `m5_proc_macros::Named` and `kit::Named`, without the leading `::`.
    fn relative_path(input: TokenStream) -> TokenStream {
        broken(input, |item| trait_path(item).leading_colon = None)
    }

    /// Ignores `#[named(crate = ..)]`.
    fn ignores_crate_argument(input: TokenStream) -> TokenStream {
        broken(input, |item| {
            *trait_path(item) = syn::parse_quote!(::m5_proc_macros::Named);
        })
    }

    /// Forgets the generic parameters after `impl`.
    fn drops_impl_generics(input: TokenStream) -> TokenStream {
        broken(input, |item| item.generics.params.clear())
    }

    /// Forgets the where clause.
    fn drops_where_clause(input: TokenStream) -> TokenStream {
        broken(input, |item| item.generics.where_clause = None)
    }

    /// Builds NAME from the printed type, generic arguments and spaces included.
    fn name_with_generics(input: TokenStream) -> TokenStream {
        broken(input, |item| {
            let printed = {
                let ty = &item.self_ty;
                quote!(#ty).to_string()
            };
            for part in &mut item.items {
                if let syn::ImplItem::Const(constant) = part {
                    constant.expr = syn::parse_quote!(#printed);
                }
            }
        })
    }

    #[test]
    fn c_the_review_accepts_your_own_implementation() {
        if let Err(complaint) = review_named(&named_impl) {
            panic!(
                "your review rejects your own `named_impl`. Either the review asks for more \
                 than contract B says, or `named_impl` has a bug. The review said: {complaint}"
            );
        }
    }

    #[test]
    fn c_the_review_rejects_every_broken_implementation() {
        let cases: [(&str, &dyn Fn(TokenStream) -> TokenStream); 5] = [
            (
                "writes the trait path without the leading `::`",
                &relative_path,
            ),
            ("ignores `#[named(crate = ..)]`", &ignores_crate_argument),
            (
                "forgets the generic parameters after `impl`",
                &drops_impl_generics,
            ),
            ("forgets the where clause", &drops_where_clause),
            ("puts the generic arguments into NAME", &name_with_generics),
        ];
        for (defect, implementation) in cases {
            assert!(
                review_named(implementation).is_err(),
                "your review accepted an implementation that {defect}. Add an input and a \
                 check that make this defect visible"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Two identifiers with the same letters and different hygiene.
        // Does `==` call them equal? (The compiler does not, when it resolves names.)
        let visible = Ident::new("tries", Span::call_site());
        let hidden = Ident::new("tries", Span::mixed_site());
        let p1: bool = guess();
        assert_eq!(
            p1,
            visible == hidden,
            "P1: `==` on identifiers that differ only in their span (left: your prediction, right: actual)"
        );

        // P2. How many token trees is this path, without the argument list?
        let path = quote!(::core::result::Result::Ok);
        let p2: usize = guess();
        assert_eq!(
            p2,
            path.into_iter().count(),
            "P2: token trees in `::core::result::Result::Ok` (left: your prediction, right: actual)"
        );

        // P3. Does this path start with `::`, and how many segments has it?
        let path: Path = syn::parse_quote!(crate::deps::kit);
        let p3: (bool, usize) = guess();
        assert_eq!(
            p3,
            (path.leading_colon.is_some(), path.segments.len()),
            "P3: (leading `::`, segments) of `crate::deps::kit` (left: your prediction, right: actual)"
        );
    }
}
