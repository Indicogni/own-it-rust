//! LESSON 02 — `syn` and `quote` for structs with named fields
//! Run: cargo test l02_
//! Read first: "Lesson 02" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. In `quote!`, what is the difference between interpolating an `Ident` and
//!    interpolating a `String` that holds the same letters?
//! 2. The generated code is pasted into the user's module. Which names in it
//!    are looked up there?
//!
//! IMPLEMENT the tasks below. Build the output with `quote!` and interpolate
//! the user's own tokens (`#ident`, `#ty`, `#vis`); do not print tokens to text
//! and parse them again. Keep the supplied tests and add one edge case.
//! EXPLAIN afterward: Follow `struct Job { id: u32 }` from tokens to a
//! `DeriveInput` to the output tokens. Which parts of the output are the
//! user's tokens, which are new, and why does that matter for error messages?
//!
//! COMPILER / TRANSFER LAB (optional): in `tests/ui/l02/pass_changes.rs`, give
//! one struct a field of a type without `PartialEq` and run `cargo test l02_`.
//! Read where the compiler points. Then compare with the recorded message in
//! `tests/ui/l02/fail_no_partial_eq.stderr`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields};

/// A. The fields of a struct with named fields, in declaration order.
/// For every other input (a tuple struct, a unit struct, an enum, a union)
/// return an empty list. Lesson 04 turns such inputs into proper errors.
pub(crate) fn named_fields(input: &DeriveInput) -> Vec<&Field> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields.named.iter().collect(),
            Fields::Unnamed(_) | Fields::Unit => Vec::new(),
        },
        Data::Enum(_) | Data::Union(_) => Vec::new(),
    }
}

/// B. `#[derive(Changes)]` for a struct with named fields and no generic
/// parameters. For `struct Job { id: u32, name: String }` return the tokens of
///
/// ```text
/// impl ::m5_proc_macros::Changes for Job {
///     fn changes(&self, __newer: &Self) -> ::std::vec::Vec<&'static str> {
///         let mut __changed = ::std::vec::Vec::new();
///         if self.id != __newer.id {
///             __changed.push("id");
///         }
///         if self.name != __newer.name {
///             __changed.push("name");
///         }
///         __changed
///     }
/// }
/// ```
///
/// (The two underscores keep the macro's own names away from the user's
/// names. Lesson 07 explains why that matters; until then, copy the habit.)
///
/// The fields are compared through references: the generated code must not
/// need `Clone` or `Copy`. A raw identifier (`r#type`) is reported without the
/// `r#` prefix (`"type"`).
///
/// Compare with `!=`, as shown. `tests/ui/l02/fail_no_partial_eq.stderr`
/// records what the compiler says about exactly this code when a field has no
/// `PartialEq`; other code that does the same work gets another message.
pub(crate) fn changes_impl(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let checks = named_fields(input).into_iter().map(|field| {
        let ident = field.ident.as_ref().expect("a named field has a name");
        // A string interpolates as a string literal. `unraw` removes `r#`.
        let text = syn::ext::IdentExt::unraw(ident).to_string();
        quote! {
            if self.#ident != __newer.#ident {
                __changed.push(#text);
            }
        }
    });
    quote! {
        impl ::m5_proc_macros::Changes for #name {
            fn changes(&self, __newer: &Self) -> ::std::vec::Vec<&'static str> {
                let mut __changed = ::std::vec::Vec::new();
                #(#checks)*
                __changed
            }
        }
    }
}

/// C. `#[derive(Setters)]` for a struct with named fields and no generic
/// parameters. It returns one inherent `impl` block with one method per field.
/// For `struct Job { pub id: u32, name: String }`:
///
/// ```text
/// impl Job {
///     pub fn set_id(&mut self, __value: u32) -> u32 {
///         ::core::mem::replace(&mut self.id, __value)
///     }
///     fn set_name(&mut self, __value: String) -> String {
///         ::core::mem::replace(&mut self.name, __value)
///     }
/// }
/// ```
///
/// Each method has the visibility of its field, so a private field does not
/// get a public setter. The method name is `set_` plus the field name without
/// any `r#` prefix. The method returns the old value.
pub(crate) fn setters_impl(input: &DeriveInput) -> TokenStream {
    let name = &input.ident;
    let setters = named_fields(input).into_iter().map(|field| {
        let ident = field.ident.as_ref().expect("a named field has a name");
        let (vis, ty) = (&field.vis, &field.ty);
        // `format_ident!` drops a leading `r#` from an identifier argument.
        let setter = format_ident!("set_{}", ident);
        quote! {
            #vis fn #setter(&mut self, __value: #ty) -> #ty {
                ::core::mem::replace(&mut self.#ident, __value)
            }
        }
    });
    quote! {
        impl #name {
            #(#setters)*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{assert_absolute, at, compact, flatten, guess, parse, tokens};
    use proc_macro2::TokenTree;
    use syn::{ImplItem, ItemImpl, Visibility};

    const JOB: &str = "
/// A unit of work.
#[derive(Debug)]
pub struct Job {
    pub id: u32,
    name: String,
    pub(crate) tags: Vec<(String, u8)>,
}";

    fn input(source: &str) -> DeriveInput {
        parse(tokens(source), "a struct, enum or union")
    }

    fn methods(item: &ItemImpl) -> Vec<&syn::ImplItemFn> {
        item.items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Fn(method) => Some(method),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_named_fields_come_in_declaration_order() {
        let job = input(JOB);
        let names: Vec<String> = named_fields(&job)
            .iter()
            .map(|field| field.ident.as_ref().expect("named").to_string())
            .collect();
        assert_eq!(names, ["id", "name", "tags"]);
        assert!(named_fields(&input("struct Empty {}")).is_empty());
    }

    #[test]
    fn a_other_shapes_have_no_named_fields() {
        for source in [
            "struct Unit;",
            "struct Pair(u8, u8);",
            "enum Level { Low, High { by: u8 } }",
            "union Bits { int: u32, float: f32 }",
        ] {
            assert!(
                named_fields(&input(source)).is_empty(),
                "`{source}` is not a struct with named fields, so the list must be empty"
            );
        }
    }

    #[test]
    fn b_changes_is_a_trait_impl_for_the_struct() {
        let output = changes_impl(&input(JOB));
        let item: ItemImpl = parse(output, "one `impl` block");
        let (path, _) = item.trait_.as_ref().expect("an impl of a trait");
        assert_eq!(
            compact(path),
            "::m5_proc_macros::Changes",
            "name the trait by its absolute path, starting with `::`"
        );
        assert_eq!(compact(&item.self_ty), "Job");
        let methods = methods(&item);
        assert_eq!(methods.len(), 1, "the trait has exactly one method");
        assert_eq!(methods[0].sig.ident, "changes");
    }

    #[test]
    fn b_field_names_become_string_literals_in_order() {
        let output = changes_impl(&input(JOB));
        let literals: Vec<String> = flatten(output)
            .into_iter()
            .filter_map(|token| match token {
                TokenTree::Literal(literal) => Some(literal.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(
            literals,
            ["\"id\"", "\"name\"", "\"tags\""],
            "each field name must appear once as a string literal, in declaration order"
        );
        let raw = changes_impl(&input("struct Token { r#type: u8 }"));
        assert!(
            compact(&raw).contains("push(\"type\")"),
            "a raw identifier is reported without `r#`; output was: {raw}"
        );
        assert!(
            compact(&raw).contains("self.r#type!=__newer.r#type"),
            "the field access must keep the raw identifier; output was: {raw}"
        );
    }

    #[test]
    fn b_generated_code_uses_absolute_paths() {
        let output = changes_impl(&input(JOB));
        assert_absolute(&output, &["Vec", "std", "core", "alloc", "m5_proc_macros"]);
    }

    #[test]
    fn b_c_field_tokens_keep_their_spans() {
        for (what, output) in [
            ("changes_impl", changes_impl(&input(JOB))),
            ("setters_impl", setters_impl(&input(JOB))),
        ] {
            for field in ["id", "name", "tags"] {
                let expected = at(JOB, &format!("{field}:"));
                let reused = flatten(output.clone()).into_iter().any(|token| {
                    let start = token.span().start();
                    matches!(&token, TokenTree::Ident(ident) if ident == field)
                        && (start.line, start.column) == expected
                });
                assert!(
                    reused,
                    "{what}: no `{field}` token in the output has the span of the field in the \
                     input (line {}, column {}). Interpolate the field's own identifier with \
                     `#ident`. Tokens that are printed to text and parsed again have lost their \
                     spans: the compiler and the editor can no longer connect the generated \
                     code to the user's source.",
                    expected.0, expected.1
                );
            }
        }
    }

    #[test]
    fn c_setters_are_inherent_methods_named_after_the_fields() {
        let output = setters_impl(&input(JOB));
        let item: ItemImpl = parse(output, "one `impl` block");
        assert!(
            item.trait_.is_none(),
            "Setters adds inherent methods (`impl Job`), not a trait impl"
        );
        assert_eq!(compact(&item.self_ty), "Job");
        let names: Vec<String> = methods(&item)
            .iter()
            .map(|method| method.sig.ident.to_string())
            .collect();
        assert_eq!(names, ["set_id", "set_name", "set_tags"]);
        let raw: ItemImpl = parse(
            setters_impl(&input("struct Token { r#type: u8 }")),
            "one `impl` block",
        );
        assert_eq!(
            methods(&raw)[0].sig.ident,
            "set_type",
            "the setter of the raw identifier `r#type` is `set_type`"
        );
    }

    #[test]
    fn c_setters_copy_visibility_and_type_from_the_field() {
        let output = setters_impl(&input(JOB));
        let item: ItemImpl = parse(output, "one `impl` block");
        let methods = methods(&item);
        assert!(
            matches!(methods[0].vis, Visibility::Public(_)),
            "`pub id` gets a `pub` setter"
        );
        assert!(
            matches!(methods[1].vis, Visibility::Inherited),
            "the private field `name` must not get a public setter"
        );
        assert_eq!(
            compact(&methods[2].vis),
            "pub(crate)",
            "`pub(crate) tags` gets a `pub(crate)` setter"
        );
        let tags = &methods[2].sig;
        assert_eq!(
            compact(&tags.inputs),
            "&mutself,__value:Vec<(String,u8)>",
            "the setter takes `&mut self` and `__value`, a value of the field's type"
        );
        assert_eq!(
            compact(&tags.output),
            "->Vec<(String,u8)>",
            "the setter returns the old value"
        );
        assert_absolute(&setters_impl(&input(JOB)), &["mem", "core", "std"]);
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    /// `compact` prints tokens and removes all white space.
    #[test]
    fn p_predictions() {
        // P1. A number is interpolated as a literal of its own type.
        let count: usize = 3;
        let p1: &str = "[0;3usize]";
        assert_eq!(
            p1,
            compact(quote!([0; #count])),
            "P1: tokens of `[0; #count]` for a usize (left: your prediction, right: actual)"
        );

        // P2. The same letters, once as an identifier and once as a string.
        let ident = format_ident!("id");
        let text = String::from("id");
        let p2: &str = "id=\"id\"";
        assert_eq!(
            p2,
            compact(quote!(#ident = #text)),
            "P2: tokens of `#ident = #text` (left: your prediction, right: actual)"
        );

        // P3. A repetition with the separator inside, and one with it outside.
        let names = [format_ident!("a"), format_ident!("b")];
        let p3: (&str, &str) = ("a,b,", "a,b");
        assert_eq!(
            p3,
            (
                compact(quote!(#(#names,)*)).as_str(),
                compact(quote!(#(#names),*)).as_str()
            ),
            "P3: tokens of `#(#names,)*` and of `#(#names),*` (left: your prediction, right: actual)"
        );

        // P4. How many token trees does this stream hold at its top level?
        // (`!=` is not one token here.)
        let field = format_ident!("id");
        let p4: usize = 8;
        assert_eq!(
            p4,
            quote!(self.#field != other.#field).into_iter().count(),
            "P4: top-level token trees in `self.id != other.id` (left: your prediction, right: actual)"
        );
    }
}
