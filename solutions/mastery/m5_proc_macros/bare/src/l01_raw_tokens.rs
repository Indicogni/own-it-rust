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
    let mut tokens = item.into_iter();
    // Attributes hide their contents inside a group and a visibility is `pub`
    // plus at most one group, so the first identifier at this level that is one
    // of the three keywords is the item's keyword.
    let kind = loop {
        if let TokenTree::Ident(ident) = tokens.next()? {
            match ident.to_string().as_str() {
                "struct" => break Kind::Struct,
                "enum" => break Kind::Enum,
                "union" => break Kind::Union,
                _ => {}
            }
        }
    };
    let TokenTree::Ident(name) = tokens.next()? else {
        return None;
    };
    let (generic, body) = match tokens.next() {
        Some(TokenTree::Punct(punct)) => (punct.as_char() == '<', None),
        Some(TokenTree::Group(group)) => (false, Some(group)),
        _ => (false, None),
    };
    Some(Item {
        kind,
        name,
        generic,
        body,
    })
}

/// B. The tokens of `::core::compile_error!("MESSAGE");`, built one by one, with
/// `span` set on every token. The compiler then reports MESSAGE at that span.
/// The message may contain any characters, including `"` and `\`.
pub(crate) fn error_at(span: Span, message: &str) -> TokenStream {
    // `Literal::string` adds the quotes and escapes the text.
    let mut argument = Literal::string(message);
    // `set_span` on a group changes only its delimiters, so the literal inside
    // the parentheses needs its span set here.
    argument.set_span(span);
    let mut tokens: Vec<TokenTree> = Vec::new();
    for segment in ["core", "compile_error"] {
        // A path separator is two `:` tokens. `Joint` on the first one says
        // that the next token follows directly, which makes the two one `::`.
        tokens.push(Punct::new(':', Spacing::Joint).into());
        tokens.push(Punct::new(':', Spacing::Alone).into());
        tokens.push(Ident::new(segment, span).into());
    }
    tokens.push(Punct::new('!', Spacing::Alone).into());
    tokens.push(Group::new(Delimiter::Parenthesis, TokenTree::from(argument).into()).into());
    tokens.push(Punct::new(';', Spacing::Alone).into());
    for token in &mut tokens {
        token.set_span(span);
    }
    tokens.into_iter().collect()
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
    let Some(item) = parse_item(item) else {
        return error_at(
            Span::call_site(),
            "`TypeName` expects a struct, enum or union",
        );
    };
    if item.generic {
        return error_at(
            item.name.span(),
            "`TypeName` (lesson 01) does not support generic types",
        );
    }
    let name = item.name.to_string();
    // The name goes through text here. That is short, and it has a price: the
    // `Job` token of the output is a new token with the span of the derive,
    // not the user's own `Job`. Lesson 02 shows why that matters and how
    // `quote!` avoids it.
    // `{text:?}` writes the name as a string literal, with quotes and escapes.
    let text = name.trim_start_matches("r#");
    format!(
        "impl ::m5_proc_macros::TypeName for {name} {{
            const TYPE_NAME: &'static str = {text:?};
        }}"
    )
    .parse()
    .expect("the generated impl is valid Rust")
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
    let mut names = Vec::new();
    let mut tokens = body.into_iter().peekable();
    loop {
        // Before the name: attributes and a visibility.
        let name = loop {
            match tokens.next() {
                None => return names,
                Some(TokenTree::Punct(punct)) if punct.as_char() == '#' => {
                    tokens.next(); // the `[...]` group of the attribute
                }
                Some(TokenTree::Ident(ident)) if ident.to_string() == "pub" => {
                    // `pub(crate)`, `pub(super)`, `pub(in some::path)`
                    if matches!(tokens.peek(), Some(TokenTree::Group(_))) {
                        tokens.next();
                    }
                }
                Some(TokenTree::Ident(ident)) => break ident,
                Some(_) => {}
            }
        };
        names.push(name);
        // After the name: skip `: Type` up to the comma that ends the field.
        // Parentheses, brackets and braces arrive as one group each, so only
        // angle brackets have to be counted by hand.
        let mut depth = 0usize;
        let mut after_minus = false;
        for token in tokens.by_ref() {
            let TokenTree::Punct(punct) = &token else {
                after_minus = false;
                continue;
            };
            match punct.as_char() {
                '<' => depth += 1,
                '>' if !after_minus => depth = depth.saturating_sub(1),
                ',' if depth == 0 => break,
                _ => {}
            }
            after_minus = punct.as_char() == '-';
        }
    }
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
    let Some(item) = parse_item(item) else {
        return error_at(Span::call_site(), "`FieldNames` expects a struct");
    };
    if item.generic {
        return error_at(
            item.name.span(),
            "`FieldNames` (lesson 01) does not support generic types",
        );
    }
    let body = match item.body {
        Some(body) if item.kind == Kind::Struct && body.delimiter() == Delimiter::Brace => body,
        _ => {
            return error_at(
                item.name.span(),
                "`FieldNames` needs a struct with named fields",
            );
        }
    };
    let names: Vec<String> = field_names(body.stream())
        .iter()
        .map(|field| field.to_string().trim_start_matches("r#").to_owned())
        .collect();
    let name = item.name;
    // `{names:?}` writes `["id", "name"]`: a valid array expression.
    format!(
        "impl ::m5_proc_macros::FieldNames for {name} {{
            const FIELD_NAMES: &'static [&'static str] = &{names:?};
        }}"
    )
    .parse()
    .expect("the generated impl is valid Rust")
}
