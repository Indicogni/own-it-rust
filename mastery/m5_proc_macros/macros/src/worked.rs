//! The worked examples of CONCEPTS.md, as real code.
//!
//! You do not need to edit this file. CONCEPTS.md shows the code between the
//! `// --- CONCEPTS ...` lines below. A code block that uses `syn` or `quote`
//! cannot be compiled by `rustdoc` on its own, so the examples are compiled
//! and tested here instead, and the last test in this file checks that the
//! text in CONCEPTS.md and this file have not drifted apart.
//!
//! The examples belong to one small story: a derive `Total` that adds up the
//! fields of a struct, and a few macros around it. They use an invented crate
//! name, `::kit`, for the crate that would hold the trait.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Data, DeriveInput, Expr, Field, Ident, ItemFn, LitInt, LitStr, Token, Type};

// --- CONCEPTS: lesson 02, input ---
const STOCK: &str = r#"
struct Stock {
    apples: u8,
    pears: u32,
}
"#;
// --- end ---

// --- CONCEPTS: lesson 02, derive ---
/// `#[derive(Total)]`, first version: an inherent method that adds every
/// field, converted to `u64` with `as`.
pub(crate) fn total_impl(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let fields = match &input.data {
        Data::Struct(data) => data.fields.iter().collect(),
        Data::Enum(_) | Data::Union(_) => Vec::new(),
    };
    // `Option<Ident>`: `None` for the fields of a tuple struct.
    let idents = fields.iter().filter_map(|field| field.ident.as_ref());
    quote! {
        impl #name {
            pub fn total(&self) -> u64 {
                0 #(+ self.#idents as u64)*
            }
        }
    }
}
// --- end ---

// --- CONCEPTS: lesson 02, output ---
const STOCK_TOTAL: &str = r#"
impl Stock {
    pub fn total(&self) -> u64 {
        0 + self.apples as u64 + self.pears as u64
    }
}
"#;
// --- end ---

// --- CONCEPTS: lesson 03, input ---
const CRATE_OF: &str = r#"
struct CrateOf<T: Clone = u32>
where
    T: Send,
{
    #[total(weight = 12)]
    boxes: T,
    loose: T,
}
"#;
// --- end ---

// --- CONCEPTS: lesson 03, derive ---
/// `#[derive(Total)]`, second version: implements the trait `::kit::Total`
/// for a struct with named fields, generic or not.
pub(crate) fn total_trait_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(name, "`Total` needs a struct"));
    };
    let mut terms = Vec::new();
    for field in &data.fields {
        let ident = &field.ident;
        let weight = weight_of(field)?;
        terms.push(quote!(+ #weight * ::kit::Total::total(&self.#ident)));
    }

    // One bound per type parameter, the way the standard derives do it.
    let mut generics = input.generics.clone();
    for parameter in generics.type_params_mut() {
        parameter.bounds.push(syn::parse_quote!(::kit::Total));
    }
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics ::kit::Total for #name #ty_generics #where_clause {
            fn total(&self) -> u64 {
                0 #(#terms)*
            }
        }
    })
}

/// The number in `#[total(weight = 12)]` on a field, or 1 without it.
fn weight_of(field: &Field) -> syn::Result<LitInt> {
    let mut weight: LitInt = syn::parse_quote!(1);
    for attr in &field.attrs {
        if !attr.path().is_ident("total") {
            continue; // somebody else's attribute
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("weight") {
                weight = meta.value()?.parse()?;
                Ok(())
            } else {
                Err(meta.error("expected `weight = number`"))
            }
        })?;
    }
    Ok(weight)
}
// --- end ---

// --- CONCEPTS: lesson 03, output ---
const CRATE_OF_TOTAL: &str = r#"
impl<T: Clone + ::kit::Total> ::kit::Total for CrateOf<T>
where
    T: Send,
{
    fn total(&self) -> u64 {
        0 + 12 * ::kit::Total::total(&self.boxes) + 1 * ::kit::Total::total(&self.loose)
    }
}
"#;
// --- end ---

// --- CONCEPTS: lesson 04, input ---
const READING: &str = r#"
struct Reading {
    count: u32,
    ratio: f64,
    samples: Vec<u8>,
    drift: f32,
}
"#;
// --- end ---

// --- CONCEPTS: lesson 04, check ---
/// Finds every field of type `f32` or `f64` and reports all of them at once.
pub(crate) fn reject_floats(input: &DeriveInput) -> syn::Result<()> {
    let Data::Struct(data) = &input.data else {
        return Ok(());
    };
    let mut errors: Option<syn::Error> = None;
    for field in &data.fields {
        let Type::Path(ty) = &field.ty else {
            continue;
        };
        if ty.path.is_ident("f32") || ty.path.is_ident("f64") {
            // The span of the error is the span of the field's type.
            let error = syn::Error::new_spanned(ty, "floating-point fields cannot be totalled");
            match &mut errors {
                Some(errors) => errors.combine(error),
                None => errors = Some(error),
            }
        }
    }
    match errors {
        Some(errors) => Err(errors),
        None => Ok(()),
    }
}

/// The whole derive: tokens in, tokens out, and no way to panic.
pub(crate) fn total_derive(input: TokenStream) -> TokenStream {
    let expanded = syn::parse2::<DeriveInput>(input).and_then(|input| {
        reject_floats(&input)?;
        total_trait_impl(&input)
    });
    match expanded {
        Ok(tokens) => tokens,
        Err(error) => error.into_compile_error(),
    }
}
// --- end ---

// --- CONCEPTS: lesson 05, attribute ---
/// `#[rename(new_name)]`: the same function under another name.
pub(crate) fn rename(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let new_name: Ident = syn::parse2(args)?;
    let mut function: ItemFn = syn::parse2(item)?;
    function.sig.ident = new_name;
    Ok(function.into_token_stream())
}
// --- end ---

// --- CONCEPTS: lesson 06, parser ---
/// One entry of `colors!`: `NAME = "#rrggbb"`.
struct Color {
    name: Ident,
    rgb: u32,
}

impl Parse for Color {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let text: LitStr = input.parse()?;
        // The tokens were fine. Now check what the string says.
        let value = text.value();
        let digits = value.strip_prefix('#').unwrap_or("");
        match u32::from_str_radix(digits, 16) {
            Ok(rgb) if digits.len() == 6 => Ok(Color { name, rgb }),
            _ => Err(syn::Error::new(
                text.span(),
                "expected a colour such as \"#1a2b3c\"",
            )),
        }
    }
}

/// `colors! { RED = "#ff0000", GREEN = "#00ff00" }` declares one `u32`
/// constant per entry.
pub(crate) fn colors(input: TokenStream) -> TokenStream {
    let entries = Punctuated::<Color, Token![,]>::parse_terminated;
    let colors = match entries.parse2(input) {
        Ok(colors) => colors,
        Err(error) => return error.into_compile_error(),
    };
    let names = colors.iter().map(|color| &color.name);
    let values = colors.iter().map(|color| color.rgb);
    quote! {
        #(pub const #names: u32 = #values;)*
    }
}
// --- end ---

// --- CONCEPTS: lesson 07, naive ---
/// `timed!(expr)`: the value of `expr` and the time it took. First attempt.
pub(crate) fn timed_naive(input: TokenStream) -> TokenStream {
    let expr: Expr = match syn::parse2(input) {
        Ok(expr) => expr,
        Err(error) => return error.into_compile_error(),
    };
    quote! {
        {
            let start = ::std::time::Instant::now();
            let value = #expr;
            (value, start.elapsed())
        }
    }
}
// --- end ---

// --- CONCEPTS: lesson 07, hygienic ---
/// `timed!(expr)`, done properly.
pub(crate) fn timed(input: TokenStream) -> TokenStream {
    let expr: Expr = match syn::parse2(input) {
        Ok(expr) => expr,
        Err(error) => return error.into_compile_error(),
    };
    quote_spanned! {Span::mixed_site()=>
        {
            let __timed_start = ::std::time::Instant::now();
            let __timed_value = #expr;
            (__timed_value, __timed_start.elapsed())
        }
    }
}
// --- end ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{at, compact, errors_in, is_clean, parse, tokens};

    fn input(source: &str) -> DeriveInput {
        parse(tokens(source), "a struct, enum or union")
    }

    #[test]
    fn lesson_02_total_expands_as_shown() {
        let output = total_impl(&input(STOCK));
        assert_eq!(compact(&output), compact(tokens(STOCK_TOTAL)));
    }

    #[test]
    fn lesson_03_total_expands_as_shown() {
        let output = total_trait_impl(&input(CRATE_OF)).expect("valid input");
        assert_eq!(compact(&output), compact(tokens(CRATE_OF_TOTAL)));
        assert!(total_trait_impl(&input("struct S { #[total(wieght = 2)] a: u8 }")).is_err());
        assert!(total_trait_impl(&input("enum E { A }")).is_err());
    }

    #[test]
    fn lesson_04_every_float_field_is_reported_at_its_type() {
        let output = total_derive(tokens(READING));
        const MESSAGE: &str = "floating-point fields cannot be totalled";
        assert_eq!(
            errors_in(output),
            [
                (MESSAGE.to_owned(), at(READING, "f64")),
                (MESSAGE.to_owned(), at(READING, "f32")),
            ]
        );
        assert!(is_clean(&total_derive(tokens(STOCK))));
        assert_eq!(errors_in(total_derive(tokens("fn f() {}"))).len(), 1);
    }

    #[test]
    fn lesson_05_rename_changes_only_the_name() {
        let before: ItemFn = syn::parse_quote! {
            /// Adds one.
            #[inline]
            pub(crate) fn add_one<T: Into<u64>>(n: T) -> u64 {
                n.into() + 1
            }
        };
        let output = rename(quote!(increment), before.to_token_stream()).expect("valid");
        let after: ItemFn = parse(output, "a function");
        assert_eq!(after.sig.ident, "increment");
        let mut expected = before;
        expected.sig.ident = after.sig.ident.clone();
        assert_eq!(after, expected, "everything except the name is unchanged");
        assert!(
            rename(
                quote!("text"),
                quote!(
                    fn f() {}
                )
            )
            .is_err()
        );
        assert!(
            rename(
                quote!(g),
                quote!(
                    struct S;
                )
            )
            .is_err()
        );
    }

    #[test]
    fn lesson_06_colors_become_constants() {
        let output = colors(quote!(RED = "#ff0000", GREEN = "#00ff00",));
        assert_eq!(
            compact(&output),
            "pubconstRED:u32=16711680u32;pubconstGREEN:u32=65280u32;"
        );
        let source = "\nRED = \"#ff0000\",\nSKY = \"blue\"";
        assert_eq!(
            errors_in(colors(tokens(source))),
            [(
                "expected a colour such as \"#1a2b3c\"".to_owned(),
                at(source, "\"blue\"")
            )]
        );
        assert_eq!(errors_in(colors(quote!(RED "#ff0000"))).len(), 1);
    }

    #[test]
    fn lesson_07_both_versions_print_alike() {
        // The two expansions differ in the names they declare and in the
        // spans of their tokens. Only the compiler can see the second
        // difference; `tests/ui/l07/` has the tests for it.
        let naive = compact(timed_naive(quote!(work())));
        let hygienic = compact(timed(quote!(work())));
        assert_eq!(
            naive
                .replace("start", "__timed_start")
                .replace("value", "__timed_value"),
            hygienic
        );
    }

    /// Every `rust,ignore` block in CONCEPTS.md must be part of this file, so
    /// that the code a learner reads is code that was compiled and tested.
    /// The comparison ignores indentation and empty lines.
    #[test]
    fn concepts_code_is_compiled_here() {
        fn normalize(text: &str) -> String {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        }
        // A checkout on Windows may have `\r\n` line ends.
        let concepts = include_str!("../../CONCEPTS.md").replace("\r\n", "\n");
        let here = normalize(include_str!("worked.rs"));
        let mut checked = 0;
        let mut rest = concepts.as_str();
        while let Some(start) = rest.find("```rust,ignore\n") {
            let body = &rest[start + "```rust,ignore\n".len()..];
            let end = body.find("\n```").expect("a code block is closed");
            let block = normalize(&body[..end]);
            assert!(
                here.contains(&block),
                "CONCEPTS.md has a `rust,ignore` block that is not in worked.rs. Either the \
                 two have drifted apart, or the block is new and has never been compiled:\n\
                 {block}"
            );
            checked += 1;
            rest = &body[end..];
        }
        assert!(
            checked >= 10,
            "expected the worked examples of lessons 02 to 07"
        );
    }
}
