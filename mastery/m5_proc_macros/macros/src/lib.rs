//! Mastery module M5: the procedural macros for lessons 02 to 08 and the code
//! review lesson.
//!
//! This file is the part of the crate that the compiler sees: one public
//! function per macro. Each one converts the compiler's `proc_macro::TokenStream`
//! into a `proc_macro2::TokenStream`, calls the lesson's function, and converts
//! the result back. You do not need to edit this file.
//!
//! Why two token types? The types of `proc_macro` work only while the compiler
//! is running a macro. The `proc_macro2` crate has the same types, and they
//! also work in an ordinary unit test. So every lesson function below takes and
//! returns `proc_macro2` tokens, and its unit tests sit next to it.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

mod c01_code_review;
mod l02_syn_quote;
mod l03_generics_attrs;
mod l04_errors;
mod l05_attribute;
mod l06_function_like;
mod l07_hygiene;
mod l08_capstone;
#[cfg(test)]
mod support;
#[cfg(test)]
mod worked;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Lesson 02. `#[derive(Changes)]` implements `m5_proc_macros::Changes`.
#[proc_macro_derive(Changes)]
pub fn derive_changes(input: TokenStream) -> TokenStream {
    // `parse_macro_input!` parses the tokens into a syntax tree. When the
    // tokens are not a struct, enum or union, it returns syn's error to the
    // compiler as a `compile_error!` and leaves this function.
    let input = parse_macro_input!(input as DeriveInput);
    l02_syn_quote::changes_impl(&input).into()
}

/// Lesson 02. `#[derive(Setters)]` adds one `set_<field>` method per field.
#[proc_macro_derive(Setters)]
pub fn derive_setters(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    l02_syn_quote::setters_impl(&input).into()
}

/// Lesson 03. `#[derive(Redacted)]` implements `m5_proc_macros::Redacted`.
///
/// `attributes(redacted)` declares a *helper attribute*: inside an item that
/// has this derive, `#[redacted(...)]` is accepted and left for the derive to
/// read.
#[proc_macro_derive(Redacted, attributes(redacted))]
pub fn derive_redacted(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    l03_generics_attrs::redacted_impl(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Lesson 04. `#[derive(Label)]` implements `m5_proc_macros::Label`.
#[proc_macro_derive(Label, attributes(label))]
pub fn derive_label(input: TokenStream) -> TokenStream {
    l04_errors::label_impl(input.into()).into()
}

/// Lesson 05. `#[logged]` or `#[logged(label = "text")]` on a function.
#[proc_macro_attribute]
pub fn logged(args: TokenStream, item: TokenStream) -> TokenStream {
    l05_attribute::logged(args.into(), item.into()).into()
}

/// Lesson 06. `flags! { pub struct Perm: u8 { READ, WRITE } }`.
#[proc_macro]
pub fn flags(input: TokenStream) -> TokenStream {
    l06_function_like::flags(input.into()).into()
}

/// Lesson 07. `attempt!(max, expr)`.
#[proc_macro]
pub fn attempt(input: TokenStream) -> TokenStream {
    l07_hygiene::attempt(input.into()).into()
}

/// Lesson 07. `#[derive(Named)]` implements `m5_proc_macros::Named`.
#[proc_macro_derive(Named, attributes(named))]
pub fn derive_named(input: TokenStream) -> TokenStream {
    l07_hygiene::named_impl(input.into()).into()
}

/// Lesson 08. `#[derive(Builder)]`.
#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    l08_capstone::builder(input.into()).into()
}

/// Code review lesson. `#[derive(Reset)]` implements `m5_proc_macros::Reset`.
#[proc_macro_derive(Reset, attributes(reset))]
pub fn derive_reset(input: TokenStream) -> TokenStream {
    c01_code_review::reset_impl(input.into()).into()
}
