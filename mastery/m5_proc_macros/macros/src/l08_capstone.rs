//! LESSON 08 — Capstone: `#[derive(Builder)]`
//! Run: cargo test l08_
//! Read first: "Lesson 08" in CONCEPTS.md (the package directory).
//!
//! This lesson has no predictions and no given helper: you get a contract and
//! one entry point. Plan before you type. Write in NOTES.md which data you
//! need about each field, which checks can fail, and what the generated code
//! looks like for the example below. Then build it in small steps, and run
//! the unit tests after each step.
//!
//! IMPLEMENT the contract below. Keep the supplied tests. Then add two cases
//! of your own to `tests/ui/l08/`: `own_pass_NAME.rs`, a program that must
//! compile and run, and `own_fail_NAME.rs`, a use of the derive that must not
//! compile, with its `.stderr` file (lesson 04, task D shows how).
//! EXPLAIN afterward: Which decisions does your macro make from *syntax*
//! alone, and where can that go wrong for a user (think of a type alias for
//! `Option`)? Which part of the generated code runs user-written expressions,
//! and how did you keep your own local variables away from them?
//!
//! COMPILER / TRANSFER LAB (optional): give a struct a field named `build`
//! and read the compiler's message. Decide what your macro should do about
//! it, and add the check with a spanned error and a compile-fail case.
//!
//! # Contract
//!
//! `#[derive(Builder)]` on a struct with named fields, with or without generic
//! parameters. The example used below:
//!
//! ```text
//! #[derive(Builder)]
//! pub struct Job<T: Clone> {
//!     id: u32,
//!     note: Option<String>,
//!     #[builder(default)]
//!     retries: u8,
//!     #[builder(default = 30)]
//!     timeout: u64,
//!     #[builder(into)]
//!     owner: String,
//!     payload: T,
//! }
//! ```
//!
//! The output is exactly three items, in this order:
//!
//! 1. The struct `JobBuilder`: the name of the input plus `Builder`, with the
//!    visibility, the generic parameters and the where clause of the input. Its
//!    fields are private.
//! 2. An inherent impl for `Job<T>` with one function,
//!    `VIS fn builder() -> JobBuilder<T>`, which returns a builder in which no
//!    field is set. VIS is the visibility of the input struct, here and below.
//! 3. An inherent impl for `JobBuilder<T>` with
//!    - one setter per field, in declaration order, named like the field:
//!      `VIS fn FIELD(self, value: VALUE) -> Self`. It stores the value and
//!      returns the builder. Calling it again replaces the value;
//!    - then `VIS fn build(self) -> ::core::result::Result<Job<T>, ::m5_proc_macros::BuildError>`.
//!
//! Every public item and function of the output has a doc comment, so that
//! the derive can be used in a crate that denies `missing_docs`.
//!
//! How one field behaves:
//!
//! - **Option field.** A field whose type is written as a path that ends in
//!   `Option<X>` (`Option<X>`, `std::option::Option<X>`,
//!   `::core::option::Option<X>`). VALUE is `X`. Never set: `None`.
//! - **Every other field.** VALUE is the field's type. Never set: the field is
//!   *missing*.
//! - `#[builder(default)]`: never set means `Default::default()` of the
//!   field's type, and the field is not missing.
//! - `#[builder(default = EXPR)]`: never set means the value of EXPR, and the
//!   field is not missing. EXPR is evaluated inside `build`, and only when the
//!   field was never set. On an Option field, EXPR has the field's type
//!   (`Option<X>`).
//! - `#[builder(into)]`: the setter takes `value: impl ::core::convert::Into<VALUE>`
//!   and converts it.
//! - One attribute may hold several arguments, and a field may have several
//!   `#[builder(...)]` attributes. Attributes with other names are ignored.
//!
//! `build` returns `Err(BuildError::new("Job", missing))` when at least one
//! field is missing, where `missing` lists the names of all missing fields in
//! declaration order (without `r#`). In that case no EXPR is evaluated.
//! Otherwise it returns `Ok` with the value.
//! The generated code needs no `Clone`, `Default` or `Debug` from field types,
//! except `Default` for a field marked `#[builder(default)]`.
//!
//! Hygiene, checked by the compiler in `tests/ui/l08/`:
//! - the output works in a module that has its own items named `Option`,
//!   `Some`, `None`, `Result`, `Ok`, `Err`, `Vec`, `String`, `Default`, `Into`
//!   or `BuildError`, and in one that has constants or unit structs with
//!   names such as `value` or `missing`;
//! - every parameter and variable that the output declares has a name that
//!   starts with `__` (`self` is not a name of this kind);
//! - EXPR sees the user's items and never a local variable of `build`. The
//!   `self` of `build` counts as one: `default = self.x` must not compile
//!   (`tests/ui/l08/fail_default_self.rs`).
//!
//! Errors. All of them are reported in one run, in source order, as
//! `compile_error!` calls and nothing else:
//! - the input is not a struct with named fields:
//!   ``` `Builder` needs a struct with named fields ``` spanned on the type's name;
//! - an argument other than `default` and `into`:
//!   ``unknown `builder` argument: expected `default`, `default = expr` or `into` ``
//!   spanned on the argument's name;
//! - `default` or `into` a second time on one field:
//!   ``duplicate `default` argument`` or ``duplicate `into` argument``, spanned
//!   on the later name;
//! - anything else that does not parse: pass on syn's error.
//!
//! The macro must never panic.

use proc_macro2::TokenStream;

/// `#[derive(Builder)]`, from the raw input tokens to the output tokens.
/// See the contract at the top of this file.
pub(crate) fn builder(input: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{
        assert_absolute, at, compact, errors_in, is_clean, parse, predicates, tokens,
    };
    use syn::{FnArg, ImplItem, Item, ItemImpl, ItemStruct, ReturnType, Visibility};

    const JOB: &str = "
/// A unit of work.
#[derive(Debug)]
pub(crate) struct Job<'a, T: Clone = u8>
where
    T: Send,
{
    id: u32,
    note: ::core::option::Option<&'a str>,
    #[builder(default)]
    retries: u8,
    #[builder(default = 30, into)]
    timeout: u64,
    #[builder(into)]
    #[allow(dead_code)]
    owner: ::std::string::String,
    payload: T,
}";

    /// The three items of the output: the builder struct, the impl for the
    /// input struct, and the impl for the builder.
    fn parts(source: &str) -> (ItemStruct, ItemImpl, ItemImpl) {
        let output = builder(tokens(source));
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let file: syn::File = parse(output, "a list of items");
        let [
            Item::Struct(builder),
            Item::Impl(for_input),
            Item::Impl(for_builder),
        ] = <[Item; 3]>::try_from(file.items).unwrap_or_else(|items| {
            panic!(
                "the output must be exactly three items, but there are {}",
                items.len()
            )
        })
        else {
            panic!("the output must be a struct, an impl and another impl, in this order");
        };
        (builder, for_input, for_builder)
    }

    fn functions(block: &ItemImpl) -> Vec<&syn::ImplItemFn> {
        block
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(function) => Some(function),
                _ => None,
            })
            .collect()
    }

    /// The type of the parameter after `self`.
    fn value_type(function: &syn::ImplItemFn) -> String {
        match function.sig.inputs.iter().nth(1) {
            Some(FnArg::Typed(value)) => compact(&value.ty),
            _ => panic!("`{}` must take `self` and one value", function.sig.ident),
        }
    }

    #[test]
    fn a_the_builder_struct_mirrors_the_input() {
        let (builder, _, _) = parts(JOB);
        assert_eq!(builder.ident, "JobBuilder");
        assert_eq!(
            compact(&builder.vis),
            "pub(crate)",
            "the builder has the input's visibility"
        );
        assert_eq!(
            compact(&builder.generics.params),
            "'a,T:Clone=u8",
            "the builder has the input's generic parameters"
        );
        assert_eq!(predicates(&builder.generics), ["T:Send"]);
        for field in &builder.fields {
            assert!(
                matches!(field.vis, Visibility::Inherited),
                "the builder's fields are private"
            );
        }
        assert!(
            builder.attrs.iter().any(|attr| attr.path().is_ident("doc")),
            "the builder struct needs a doc comment (`#[doc = \"...\"]`)"
        );
    }

    #[test]
    fn a_the_input_gets_a_builder_function() {
        let (_, for_input, _) = parts(JOB);
        assert!(for_input.trait_.is_none(), "an inherent impl");
        assert_eq!(compact(&for_input.generics.params), "'a,T:Clone");
        assert_eq!(compact(&for_input.self_ty), "Job<'a,T>");
        assert_eq!(predicates(&for_input.generics), ["T:Send"]);
        let functions = functions(&for_input);
        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].sig.ident, "builder");
        assert_eq!(compact(&functions[0].vis), "pub(crate)");
        assert!(
            functions[0].sig.inputs.is_empty(),
            "`builder()` takes no arguments"
        );
        assert_eq!(compact(&functions[0].sig.output), "->JobBuilder<'a,T>");
        assert!(
            functions[0]
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("doc"))
        );
    }

    #[test]
    fn a_setters_follow_the_fields_and_build_comes_last() {
        let (_, _, for_builder) = parts(JOB);
        assert_eq!(compact(&for_builder.generics.params), "'a,T:Clone");
        assert_eq!(compact(&for_builder.self_ty), "JobBuilder<'a,T>");
        assert_eq!(predicates(&for_builder.generics), ["T:Send"]);
        let functions = functions(&for_builder);
        let names: Vec<String> = functions.iter().map(|f| f.sig.ident.to_string()).collect();
        assert_eq!(
            names,
            [
                "id", "note", "retries", "timeout", "owner", "payload", "build"
            ]
        );
        for function in &functions {
            assert_eq!(
                compact(&function.vis),
                "pub(crate)",
                "`{}` has the visibility of the input struct",
                function.sig.ident
            );
            assert!(
                function
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("doc")),
                "`{}` needs a doc comment",
                function.sig.ident
            );
        }
        let build = functions.last().expect("build");
        assert_eq!(
            compact(&build.sig.output),
            "->::core::result::Result<Job<'a,T>,::m5_proc_macros::BuildError>"
        );
    }

    #[test]
    fn a_setter_values_have_the_right_types() {
        let (_, _, for_builder) = parts(JOB);
        let functions = functions(&for_builder);
        let types: Vec<String> = functions[..6].iter().map(|f| value_type(f)).collect();
        assert_eq!(
            types,
            [
                "u32",
                "&'astr",
                "u8",
                "impl::core::convert::Into<u64>",
                "impl::core::convert::Into<::std::string::String>",
                "T"
            ],
            "an Option field takes the inner type, and `into` takes `impl Into<VALUE>`"
        );
        for setter in &functions[..6] {
            assert!(
                matches!(&setter.sig.output, ReturnType::Type(_, ty) if compact(ty) == "Self"),
                "`{}` returns `Self`, so that calls can be chained",
                setter.sig.ident
            );
        }
    }

    #[test]
    fn a_option_fields_are_found_by_their_last_path_segment() {
        let (_, _, for_builder) = parts(
            "struct Paths {
                a: Option<u8>,
                b: std::option::Option<u16>,
                c: ::core::option::Option<Vec<u32>>,
                d: Vec<Option<u8>>,
                e: (Option<u8>,),
            }",
        );
        let functions = functions(&for_builder);
        let types: Vec<String> = functions[..5].iter().map(|f| value_type(f)).collect();
        assert_eq!(
            types,
            ["u8", "u16", "Vec<u32>", "Vec<Option<u8>>", "(Option<u8>,)"],
            "only a type that itself ends in `Option<X>` is an Option field"
        );
    }

    #[test]
    fn a_library_names_are_written_as_absolute_paths() {
        let output = builder(tokens(JOB));
        assert_absolute(
            &output,
            &[
                "Option",
                "Some",
                "None",
                "Result",
                "Ok",
                "Err",
                "Vec",
                "String",
                "Default",
                "Into",
                "BuildError",
                "core",
                "std",
                "m5_proc_macros",
            ],
        );
    }

    #[test]
    fn a_declared_names_start_with_two_underscores() {
        /// Collects the names that patterns bind: parameters, `let`, `match`.
        #[derive(Default)]
        struct Bound(Vec<String>);
        impl<'ast> syn::visit::Visit<'ast> for Bound {
            fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
                self.0.push(pattern.ident.to_string());
                syn::visit::visit_pat_ident(self, pattern);
            }
        }

        // No `default = EXPR` here, so every pattern in the output is the macro's.
        let output = builder(tokens(
            "struct Job { id: u32, name: String, note: Option<u8>, #[builder(default, into)] retries: u8 }",
        ));
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        let file: syn::File = parse(output, "a list of items");
        let mut bound = Bound::default();
        syn::visit::Visit::visit_file(&mut bound, &file);
        assert!(
            !bound.0.is_empty(),
            "the setters take a value, so there are parameters"
        );
        for name in &bound.0 {
            assert!(
                name.starts_with("__"),
                "the output declares `{name}`. In a pattern, a plain name such as `{name}` \
                 means the user's constant or unit struct of that name, when there is one \
                 (lesson 07). Start every parameter and variable name with `__`"
            );
        }
    }

    #[test]
    fn b_other_shapes_are_rejected_at_the_name() {
        for (source, name) in [
            ("\nstruct Pair(u8, u8);", "Pair"),
            ("\nstruct Unit;", "Unit"),
            ("\nenum Level { Low, High }", "Level"),
            ("\nunion Bits { int: u32, float: f32 }", "Bits"),
        ] {
            assert_eq!(
                errors_in(builder(tokens(source))),
                [(
                    "`Builder` needs a struct with named fields".to_owned(),
                    at(source, name)
                )],
                "for `{source}`"
            );
        }
    }

    #[test]
    fn b_all_argument_problems_are_reported_in_order() {
        let source = "
struct Job {
    #[builder(defualt, into)]
    id: u32,
    #[builder(default = 1, each = \"x\", default)]
    #[builder(into)]
    #[builder(into)]
    retries: u8,
}";
        const UNKNOWN: &str =
            "unknown `builder` argument: expected `default`, `default = expr` or `into`";
        let output = builder(tokens(source));
        assert_eq!(
            errors_in(output.clone()),
            [
                (UNKNOWN.to_owned(), at(source, "defualt")),
                (UNKNOWN.to_owned(), at(source, "each")),
                (
                    "duplicate `default` argument".to_owned(),
                    at(source, "default)]")
                ),
                (
                    "duplicate `into` argument".to_owned(),
                    at(source, "into)]\n    retries")
                ),
            ],
            "one run must show every problem, each at the argument's name"
        );
        assert!(
            !compact(&output).contains("struct"),
            "on an error the output is the `compile_error!` calls only; output was: {output}"
        );
    }

    #[test]
    fn b_never_panics() {
        for source in [
            "",
            "fn f() {}",
            "struct S { #[builder] a: u8 }",
            "struct S { #[builder(default = )] a: u8 }",
            "struct S { #[builder = \"into\"] a: u8 }",
        ] {
            let output =
                std::panic::catch_unwind(|| builder(tokens(source))).unwrap_or_else(|_| {
                    panic!("builder panicked on `{source}`: return a compile_error! instead")
                });
            assert!(
                !errors_in(output.clone()).is_empty(),
                "`{source}` must be an error; output was: {output}"
            );
        }
    }
}
