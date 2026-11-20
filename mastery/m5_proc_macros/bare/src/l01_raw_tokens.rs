//! LESSON 01 — A derive with nothing but `proc_macro`
//! Run: cargo test l01_
//! Read first: "Lesson 01" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `l01_p_predictions` at the bottom of
//! `tests/l01_raw_tokens.rs` (replace each `guess()` with your answer). Then consider:
//! 1. `<` and `>` are not brackets for the tokenizer. What does that mean for a
//!    field of type `HashMap<String, u32>` when you split a struct body at commas?
//! 2. A derive cannot change the item it is written on. What can it do instead?
//!
//! IMPLEMENT the tasks below with the `proc_macro` crate only: this crate has
//! no dependencies, and a test checks that it stays that way. A macro must
//! never panic on input it does not like: report the problem with `error_at`.
//! The tests are in `tests/l01_raw_tokens.rs` and `tests/ui/l01/`, because the
//! types of `proc_macro` work only while the compiler is running your macro.
//! Keep the supplied tests and add one case of your own to `tests/ui/l01/`.
//! EXPLAIN afterward: What does a derive macro receive, what does it return,
//! and where does the returned code go? Why did splitting at commas need extra
//! care, and what does that tell you about tokens and syntax?
//!
//! COMPILER / TRANSFER LAB (optional): make `derive_type_name` panic on purpose
//! with `panic!("boom")`, run `cargo test l01_a`, and read what the user of
//! your macro would see. Then return the tokens of `struct Extra;` in addition
//! to the impl and check that a user can name `Extra`: derive output is
//! ordinary code next to the item.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

/// What kind of item the derive was written on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Struct,
    Enum,
    Union,
}

/// The parts of an item declaration that this lesson's derives need.
pub(crate) struct Item {
    pub kind: Kind,
    /// The item's name: the very token from the user's source, with its span.
    pub name: Ident,
    /// True when the token after the name is `<`: the item has generic parameters.
    pub generic: bool,
    /// The token after the name, when that token is a group: `{ ... }` for a
    /// struct with named fields, an enum or a union, `( ... )` for a tuple
    /// struct. `None` for `struct Unit;` and for generic items.
    pub body: Option<Group>,
}

/// A. Find the keyword `struct`, `enum` or `union` and the name after it.
/// Before the keyword there may be attributes (each one is a `#` followed by a
/// `[...]` group; a doc comment arrives as `#[doc = "..."]`) and a visibility
/// (`pub`, or `pub` followed by a `(...)` group). Return `None` when the tokens
/// are not such an item.
pub(crate) fn parse_item(item: TokenStream) -> Option<Item> {
    todo!("Implement this task")
}

/// B. The tokens of `::core::compile_error!("MESSAGE");`, built one by one, with
/// `span` set on every token. The compiler then reports MESSAGE at that span.
/// The message may contain any characters, including `"` and `\`.
pub(crate) fn error_at(span: Span, message: &str) -> TokenStream {
    todo!("Implement this task")
}

/// C. `#[derive(TypeName)]` for a struct, enum or union without generic
/// parameters. For `struct Job { .. }` return the tokens of
///
/// ```text
/// impl ::m5_proc_macros::TypeName for Job {
///     const TYPE_NAME: &'static str = "Job";
/// }
/// ```
///
/// Errors, reported with `error_at`:
/// - a generic item: ``` `TypeName` (lesson 01) does not support generic types ```
///   at the item's name;
/// - `parse_item` returned `None`: ``` `TypeName` expects a struct, enum or union ```
///   at `Span::call_site()`.
pub(crate) fn derive_type_name(item: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

/// D. The field names in the body of a struct with named fields, in declaration
/// order. `body` is the token stream *inside* the braces, for example
/// `pub id: u32, tags: HashMap<String, Vec<u8>>, run: fn(u8) -> u8`.
///
/// A field is: attributes, an optional visibility, a name, `:`, a type. Fields
/// are separated by commas, and the last comma is optional. A comma between
/// `<` and `>` belongs to a type and does not end the field. The `>` of `->`
/// does not close a `<`.
pub(crate) fn field_names(body: TokenStream) -> Vec<Ident> {
    todo!("Implement this task")
}

/// E. `#[derive(FieldNames)]` for a struct with named fields and without
/// generic parameters. For `struct Job { id: u32, name: String }` return the
/// tokens of
///
/// ```text
/// impl ::m5_proc_macros::FieldNames for Job {
///     const FIELD_NAMES: &'static [&'static str] = &["id", "name"];
/// }
/// ```
///
/// A field written as a raw identifier (`r#type`) is listed without the `r#`
/// prefix (`"type"`). Errors, reported with `error_at`:
/// - a generic item: ``` `FieldNames` (lesson 01) does not support generic types ```
///   at the item's name;
/// - anything that is not a struct with a `{ ... }` body:
///   ``` `FieldNames` needs a struct with named fields ``` at the item's name;
/// - `parse_item` returned `None`: ``` `FieldNames` expects a struct ``` at
///   `Span::call_site()`.
pub(crate) fn derive_field_names(item: TokenStream) -> TokenStream {
    todo!("Implement this task")
}
