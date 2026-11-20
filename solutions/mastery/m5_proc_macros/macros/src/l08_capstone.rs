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

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Expr, Field, Fields, GenericArgument, Ident, PathArguments, Token, Type,
};

/// Collects problems, so that one run reports all of them.
#[derive(Default)]
struct Problems(Option<syn::Error>);

impl Problems {
    fn push(&mut self, error: syn::Error) {
        match &mut self.0 {
            Some(errors) => errors.combine(error),
            None => self.0 = Some(error),
        }
    }
}

/// What `build` uses for a field that was never set.
enum Unset {
    /// Nothing: the field is missing, and `build` fails.
    Missing,
    /// The field's value as it is in the builder: `None`. Option fields only.
    Nothing,
    /// `Default::default()`.
    Default,
    /// The expression from `default = EXPR`.
    Expr(Box<Expr>),
}

/// Everything the generator needs to know about one field.
struct Plan<'a> {
    ident: &'a Ident,
    /// The field's type, as written.
    ty: &'a Type,
    /// The type of the setter's value: `X` for `Option<X>`, else `ty`.
    value: &'a Type,
    /// The field's type is written as `Option<X>`.
    optional: bool,
    into: bool,
    unset: Unset,
}

/// `Some(X)` when `ty` is written as a path that ends in `Option<X>`.
///
/// A macro sees tokens, not types. An alias for `Option` is not recognised,
/// and a user's own type named `Option` is taken for the standard one.
fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let last = path.path.segments.last()?;
    let PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return None;
    };
    match (
        last.ident == "Option",
        arguments.args.len(),
        arguments.args.first(),
    ) {
        (true, 1, Some(GenericArgument::Type(inner))) => Some(inner),
        _ => None,
    }
}

fn plan<'a>(field: &'a Field, problems: &mut Problems) -> Option<Plan<'a>> {
    let ident = field.ident.as_ref()?;
    let inner = option_inner(&field.ty);
    let mut into = false;
    let mut default: Option<Unset> = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("builder") {
            continue;
        }
        let parsed = attr.parse_nested_meta(|meta| {
            let has_value = meta.input.peek(Token![=]);
            let known = meta.path.is_ident("default") || meta.path.is_ident("into");
            let duplicate = (meta.path.is_ident("default") && default.is_some())
                || (meta.path.is_ident("into") && into);
            if !known || duplicate {
                problems.push(meta.error(if duplicate {
                    format!("duplicate `{}` argument", meta.path.require_ident()?)
                } else {
                    "unknown `builder` argument: expected `default`, `default = expr` or `into`"
                        .to_owned()
                }));
                // Read the value, if there is one, so that the arguments after
                // this one are still checked.
                if has_value {
                    meta.value()?.parse::<Expr>()?;
                }
            } else if meta.path.is_ident("into") {
                into = true;
            } else if has_value {
                default = Some(Unset::Expr(Box::new(meta.value()?.parse()?)));
            } else {
                default = Some(Unset::Default);
            }
            Ok(())
        });
        if let Err(error) = parsed {
            problems.push(error);
        }
    }
    let unset = default.unwrap_or(if inner.is_some() {
        Unset::Nothing
    } else {
        Unset::Missing
    });
    Some(Plan {
        ident,
        ty: &field.ty,
        value: inner.unwrap_or(&field.ty),
        optional: inner.is_some(),
        into,
        unset,
    })
}

fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let Data::Struct(syn::DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = &input.data
    else {
        return Err(syn::Error::new(
            name.span(),
            "`Builder` needs a struct with named fields",
        ));
    };

    let mut problems = Problems::default();
    let plans: Vec<Plan> = fields
        .named
        .iter()
        .filter_map(|field| plan(field, &mut problems))
        .collect();
    if let Some(errors) = problems.0 {
        return Err(errors);
    }

    let vis = &input.vis;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let builder = format_ident!("{}Builder", name);
    let name_text = name.unraw().to_string();
    let idents: Vec<&Ident> = plans.iter().map(|plan| plan.ident).collect();

    // --- the builder struct: every field starts as `None` ---
    let stored = plans.iter().map(|plan| {
        let (ident, value) = (plan.ident, plan.value);
        quote!(#ident: ::core::option::Option<#value>)
    });

    // --- the setters ---
    let setters = plans.iter().map(|plan| {
        let (ident, value) = (plan.ident, plan.value);
        let doc = format!("Sets `{}`.", ident.unraw());
        if plan.into {
            quote! {
                #[doc = #doc]
                #vis fn #ident(mut self, __value: impl ::core::convert::Into<#value>) -> Self {
                    self.#ident =
                        ::core::option::Option::Some(::core::convert::Into::into(__value));
                    self
                }
            }
        } else {
            quote! {
                #[doc = #doc]
                #vis fn #ident(mut self, __value: #value) -> Self {
                    self.#ident = ::core::option::Option::Some(__value);
                    self
                }
            }
        }
    });

    // --- build ---
    // `default = EXPR` puts a user's expression into the body of `build`. So
    // the whole function is written with the span `mixed_site`: its local
    // variables, and its `self`, can neither be read by EXPR nor change what
    // EXPR means (lesson 07). Interpolated tokens keep their own spans. The
    // names start with `__` as well, because a pattern can also name a
    // constant or a unit struct of the user, and hygiene does not cover items.
    let mixed = Span::mixed_site();
    let required: Vec<&Plan> = plans
        .iter()
        .filter(|plan| matches!(plan.unset, Unset::Missing))
        .collect();
    let required_idents: Vec<&Ident> = required.iter().map(|plan| plan.ident).collect();
    let required_texts = required.iter().map(|plan| plan.ident.unraw().to_string());
    let required_locals: Vec<Ident> = (0..required.len())
        .map(|index| Ident::new(&format!("__required_{index}"), mixed))
        .collect();
    // With no required field there is nothing to check, and a `let ... else`
    // whose pattern always matches would be a warning.
    let check = if required.is_empty() {
        quote!()
    } else {
        quote_spanned! {mixed=>
            let mut __missing = ::std::vec::Vec::new();
            #(
                if self.#required_idents.is_none() {
                    __missing.push(#required_texts);
                }
            )*
            let (#(::core::option::Option::Some(#required_locals),)*) = (#(self.#required_idents,)*)
            else {
                return ::core::result::Result::Err(
                    ::m5_proc_macros::BuildError::new(#name_text, __missing),
                );
            };
        }
    };
    let mut locals = required_locals.iter();
    let values = plans.iter().map(|plan| {
        let ident = plan.ident;
        let fallback = match &plan.unset {
            Unset::Missing => {
                let local = locals.next();
                return quote!(#ident: #local);
            }
            Unset::Nothing => return quote_spanned!(mixed=> #ident: self.#ident),
            // With the span of the field's type, a type without `Default` is
            // reported at the field and not at the derive attribute.
            Unset::Default => {
                quote_spanned!(plan.ty.span()=> ::core::default::Default::default())
            }
            Unset::Expr(expr) => quote!(#expr),
        };
        // The builder stores `Option<X>` for an Option field too, so a value
        // that was set has to be wrapped again.
        let set = if plan.optional {
            quote_spanned!(mixed=> ::core::option::Option::Some(__value))
        } else {
            quote_spanned!(mixed=> __value)
        };
        // The fallback is first stored in a variable of the field's type.
        // If EXPR has another type, the compiler then says "expected `u64`,
        // found `&str`" and points at EXPR and at the field's type, instead of
        // talking about the arms of a `match` that the user never wrote.
        let ty = plan.ty;
        quote_spanned! {mixed=>
            #ident: match self.#ident {
                ::core::option::Option::Some(__value) => #set,
                ::core::option::Option::None => {
                    let __fallback: #ty = #fallback;
                    __fallback
                }
            }
        }
    });
    let build_doc =
        format!("Makes the `{name_text}`, or reports every required field that was never set.");
    let build = quote_spanned! {mixed=>
        #[doc = #build_doc]
        #vis fn build(
            self,
        ) -> ::core::result::Result<#name #ty_generics, ::m5_proc_macros::BuildError> {
            #check
            ::core::result::Result::Ok(#name {
                #(#values,)*
            })
        }
    };

    let builder_doc = format!("Builder for [`{name_text}`]. Made by `{name_text}::builder()`.");
    let new_doc = format!("Returns a builder for `{name_text}` in which no field is set.");
    Ok(quote! {
        #[doc = #builder_doc]
        #vis struct #builder #generics #where_clause {
            #(#stored,)*
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #[doc = #new_doc]
            #vis fn builder() -> #builder #ty_generics {
                #builder {
                    #(#idents: ::core::option::Option::None,)*
                }
            }
        }

        impl #impl_generics #builder #ty_generics #where_clause {
            #(#setters)*

            #build
        }
    })
}

/// `#[derive(Builder)]`, from the raw input tokens to the output tokens.
/// See the contract at the top of this file.
pub(crate) fn builder(input: TokenStream) -> TokenStream {
    expand(input).unwrap_or_else(syn::Error::into_compile_error)
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
