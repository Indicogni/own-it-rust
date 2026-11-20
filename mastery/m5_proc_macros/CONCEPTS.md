# Mastery M5 concepts — Procedural macros

[Module guide](README.md) · [Hints](HINTS.md) · [All chapters](../../README.md)

Read one section, then do the lesson with the same number. Each section is the
teaching text for that lesson. Every code block in this file was compiled with
the course toolchain (Rust 1.99.0, edition 2024) and the crate versions pinned
in this package (syn 3.0.10, quote 1.0.47, proc-macro2 1.0.107, trybuild
1.0.125). Every compiler message was copied from that compiler. Long messages
are shortened: a line with `...` marks a cut.

Two kinds of code block appear. A block that needs only the standard library
is compiled when you run `cargo test --doc`. A block that uses `syn` or `quote`
is taken, line for line, from `macros/src/worked.rs`, where it is compiled and
tested; a test in that file fails if the two ever differ.

## What you will be able to do

After this module you can:

- say what a procedural macro receives, what it returns, and at which moment
  of a build it runs;
- write a derive, an attribute macro and a function-like macro, first with the
  compiler's own `proc_macro` crate and then with `syn` and `quote`;
- carry generic parameters and where clauses into generated code, and read
  helper attributes;
- report mistakes as compiler errors that point at the wrong token, and keep
  those errors from changing unnoticed with a compile-fail test suite;
- generate code that means the same thing in every module it is expanded in;
- choose the right kind of test for each property of a macro.

## Before you start

Chapters 1 to 22. In particular: traits, generics and
where clauses (Chapters 11 and 13), error handling with `Result` and `?`
(Chapter 7), crates, Cargo and workspaces (Chapter 8), and `macro_rules!`
(Chapter 22), because this module often compares the two kinds of macro.
Lesson 05 has one `async fn` in a test; you do not need Chapter 20 to do it.

## Time

These are estimates for a learner who has finished the chapters
above. They include reading the section, the predictions, the code, and your
notes.

| Lesson | Estimate |
| --- | --- |
| 01 · A derive with nothing but `proc_macro` | 90 minutes |
| 02 · `syn` and `quote` for structs with named fields | 75 minutes |
| 03 · Generics, where clauses and helper attributes | 90 minutes |
| 04 · Errors that point at the right place, and a compile-fail suite | 100 minutes |
| 05 · An attribute macro that keeps the item intact | 75 minutes |
| 06 · A function-like macro with its own syntax | 100 minutes |
| 07 · Hygiene, crate paths, and how to test a macro | 120 minutes |
| 08 · Capstone: `#[derive(Builder)]` | 3 to 4 hours |
| C01 code review, drills | 2 hours |

## Words used in this module

- A **token** is the smallest piece of source text that the compiler works
  with: a name, a punctuation character, a literal, or a bracketed group.
- A **token stream** is a sequence of tokens. It has no meaning yet: it is not
  known to be a struct or an expression.
- A **span** is the place in a source file that a token came from. Every token
  carries one. The compiler uses spans to underline code in its messages, and
  to decide which names a token may refer to.
- To **expand** a macro is to run it and put its output in place. The output
  is the macro's **expansion**.
- The **user** of a macro is the programmer who writes `#[derive(YourMacro)]`
  in their own crate. This module calls that crate the user's crate, and its
  code the **call site**.

## Lesson 01 · A derive with nothing but `proc_macro`

**Assumed:** `macro_rules!` from Chapter 22, and workspaces from Chapter 8.

### The mental model

A `macro_rules!` macro is a list of patterns. A procedural macro is a
function. You write it in ordinary Rust, the compiler compiles it, and then
the compiler *calls* it while it compiles some other crate. The function
receives tokens and returns tokens. That is the whole interface.

Three consequences follow from "the compiler calls it":

1. The macro must be compiled before the crate that uses it. So it lives in a
   crate of its own, marked `proc-macro = true` in `Cargo.toml`. Such a crate
   can export macros and nothing else.
2. The macro runs on the machine that builds, with all the rights of the
   compiler. It can read files and take as long as it likes. A good macro
   does neither: it is a pure function from tokens to tokens.
3. The macro sees *syntax only*. Types have not been checked when it runs. It
   cannot ask whether a field's type implements a trait, or what a type alias
   stands for.

There are three kinds of procedural macro, and they differ only in what
happens to the output:

| Kind | Written by the user as | Input | The output... |
| --- | --- | --- | --- |
| derive | `#[derive(Name)]` on a struct, enum or union | the item | is added after the item |
| attribute | `#[name(args)]` on any item | the arguments, and the item | replaces the item |
| function-like | `name!(...)` | the tokens between the delimiters | replaces the call |

This lesson uses the first kind. A derive cannot change the item it is
written on. It can only add new items next to it, usually an `impl` block.
The input is the item with its attributes (doc comments are attributes). The
compiler removes the `#[derive(...)]` attribute that names your macro before
it calls you. A `#[derive(...)]` attribute written *below* that one is still
in the tokens, so a hand-written parser must skip attributes and may not
assume that every derive line is gone.

### Tokens are trees

The type `proc_macro::TokenStream` is an iterator over `TokenTree` values. A
`TokenTree` is one of four things:

- `Ident`: a name or a keyword, such as `struct`, `Job` or `r#type`;
- `Punct`: *one* punctuation character, such as `:` or `<`;
- `Literal`: a number, string or character literal;
- `Group`: a pair of delimiters, `( )`, `[ ]` or `{ }`, with a token stream
  inside.

Here is what a macro receives for a small struct. The listing was printed by
a macro that walks its input.

```text
/// A label.
pub(crate) struct Tag { name: Option<&'static str>, size: u8 }

Punct  #                         a doc comment arrives as #[doc = r" A label."]
Group  [ ]
    Ident    doc
    Punct    =
    Literal  r" A label."
Ident  pub
Group  ( )
    Ident    crate
Ident  struct
Ident  Tag
Group  { }
    Ident    name
    Punct    :
    Ident    Option
    Punct    <       Joint       followed directly by another punctuation character
    Punct    &
    Punct    '       Joint       a lifetime is ' and then an Ident
    Ident    static
    Ident    str
    Punct    >       Joint
    Punct    ,
    Ident    size
    Punct    :
    Ident    u8
```

Two facts in this listing decide how lesson 01 goes.

First, brackets make groups, and a group is *one* tree. When you walk the
top level, you step over a whole `{ ... }` body, or a whole attribute, in one
step. Whatever is inside cannot confuse you.

Second, `<` and `>` are not brackets. To the tokenizer they are plain
characters, because it cannot know whether `<` opens a list of type arguments
or means "less than". So the comma inside `HashMap<String, u32>` sits at the
same level as the comma that ends the field. Multi-character operators are
split up as well: `->` is the two tokens `-` and `>`, and the first one is
marked `Spacing::Joint`, which means "the next punctuation character follows
without a space".

### A worked example

A function-like macro is the shortest way to see the interface. `shout!`
turns words into one upper-case string literal:

```rust
extern crate proc_macro;
use proc_macro::{Literal, TokenStream, TokenTree};

/// `shout!(hello big world)` expands to the literal `"HELLO BIG WORLD"`.
/// In a macro crate, this function has the attribute `#[proc_macro]`.
pub fn shout(input: TokenStream) -> TokenStream {
    let mut words = Vec::new();
    for token in input {
        match token {
            TokenTree::Ident(word) => words.push(word.to_string().to_uppercase()),
            _ => {
                return "::core::compile_error!(\"shout! takes plain words only\")"
                    .parse()
                    .unwrap();
            }
        }
    }
    TokenTree::from(Literal::string(&words.join(" "))).into()
}

// Nothing to run: the compiler calls `shout`, a program never does.
fn main() {}
```

The function shows the two ways to make tokens. `Literal::string` builds one
token by hand, and `From`/`Into` wraps it into a tree and then into a stream.
The error branch writes source text and calls `str::parse`, which runs the
tokenizer on it. Parsing text is short to write. It is safe here because the
text is fixed; lesson 02 shows what goes wrong when the text contains the
user's tokens.

The error branch also shows how a macro reports a problem: it *succeeds*, and
returns a call of the standard macro `compile_error!`. The compiler then
expands that call and prints the message:

```text
error: shout! takes plain words only
 --> use/src/main.rs:1:37
  |
1 | use mac::shout; fn main() { let _ = shout!(hello, world); }
  |                                     ^^^^^^^^^^^^^^^^^^^^
```

The message points at the whole macro call, because tokens that come from
`parse` have the span of the call. In task B of the lesson you build the same
`compile_error!` call token by token, and give each token a span that you
choose. Then the compiler underlines exactly that place.

### Mistakes people really make

**The macro panics.** A panic in a procedural macro does not crash the
compiler. The user gets this, with the panic message as a note:

```text
error: proc-macro derive panicked
 --> use/src/main.rs:1:25
  |
1 | use mac::Boom; #[derive(Boom)] struct Job; fn main(){}
  |                         ^^^^
  |
  = help: message: boom
```

It always points at the derive, never at the mistake, and it reads like a bug
in your macro. Use a panic (`unwrap`, `expect`, indexing) only for things that
cannot happen. For input that the user may really write, return a
`compile_error!`.

**A helper function is made public.** A crate with `proc-macro = true`
exports macros only:

```text
error: `proc-macro` crate types currently cannot export any items other than functions tagged with `#[proc_macro]`, `#[proc_macro_derive]`, or `#[proc_macro_attribute]`
  --> mac/src/lib.rs:96:1
   |
96 | pub fn helper() -> u32 { 1 }
   | ^^^^^^^^^^^^^^^^^^^^^^
```

Use `pub(crate)` for helpers. This rule is also why a trait and its derive
are always in two crates: the trait cannot live in the macro crate. This
package has that shape. `m5_proc_macros` holds the traits and re-exports the
derives; `bare/` and `macros/` hold the macros.

**Splitting at every comma.** `a: HashMap<String, u32>, b: u8` has two
commas at the top level and two fields. Count angle brackets, and remember
that the `>` of `->` closes nothing.

### When to use which

Writing a macro with `proc_macro` alone means writing a small parser by hand,
as you are about to do. It is the right choice for a macro with a tiny input
grammar that should compile fast and have no dependencies. For anything that
must understand Rust syntax in general (generics, where clauses, all forms of
types), use `syn`, from lesson 02 on. The cost of `syn` is compile time for
every user of your macro; the benefit is that it is correct for syntax you
never thought of.

### Read more

- [Procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html) in the Rust Reference
- [The `proc_macro` crate](https://doc.rust-lang.org/proc_macro/index.html), in particular [`TokenTree`](https://doc.rust-lang.org/proc_macro/enum.TokenTree.html) and [`Spacing`](https://doc.rust-lang.org/proc_macro/enum.Spacing.html)
- [`compile_error!`](https://doc.rust-lang.org/core/macro.compile_error.html)
- [Tokens](https://doc.rust-lang.org/reference/tokens.html) in the Rust Reference

## Lesson 02 · `syn` and `quote` for structs with named fields

**Assumed:** lesson 01: token streams, token trees, spans, and how a derive
is exported.

### The mental model

Lesson 01 had two hard parts: understanding the input, and writing the
output. Two crates take over one part each.

- **`syn`** parses a token stream into a syntax tree: typed Rust values that
  describe a struct, a field, a type, an attribute. The tree for the input of
  a derive is `syn::DeriveInput`.
- **`quote`** goes the other way. The macro `quote!` lets you write the output
  as Rust code and *interpolate* values into it with `#name`.

Both crates work on the types of a third crate, **`proc_macro2`**. It has the
same `TokenStream`, `Ident`, `Span` and so on as the compiler's `proc_macro`,
with one difference: the compiler's types work only while the compiler is
running a macro, and the `proc_macro2` types also work in a unit test. So a
macro crate has this shape:

```text
#[proc_macro_derive(Total)]                      macros/src/lib.rs
pub fn derive_total(input: proc_macro::TokenStream) -> proc_macro::TokenStream
        |  parse_macro_input!                 ^
        v                                     |  .into()
   syn::DeriveInput  -->  your function  -->  proc_macro2::TokenStream
                          (unit-tested)
```

The exported function is three lines. Your logic takes a syntax tree and
returns `proc_macro2` tokens, and its tests sit next to it. Every lesson file
in `macros/src/` is built this way.

### The tree

For this input

```rust,ignore
struct Stock {
    apples: u8,
    pears: u32,
}
```

`syn` produces a `DeriveInput`. The parts you need most often:

```text
DeriveInput
├── attrs: []                      attributes on the struct (doc comments too)
├── vis: Visibility::Inherited     no `pub`
├── ident: Stock
├── generics: (empty)              lesson 03
└── data: Data::Struct
    └── fields: Fields::Named      the other shapes: Unnamed (tuple), Unit
        ├── Field { attrs, vis, ident: Some(apples), ty: u8 }
        └── Field { attrs, vis, ident: Some(pears), ty: u32 }
```

`Data` is an enum with the variants `Struct`, `Enum` and `Union`, and `Fields`
is an enum too. A derive decides early which shapes it supports. `ident` is an
`Option` because the fields of a tuple struct have no names.

### A worked example

`#[derive(Total)]` gives a struct a method that adds up its fields:

```rust,ignore
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
```

For `Stock`, the function returns these tokens:

```rust,ignore
impl Stock {
    pub fn total(&self) -> u64 {
        0 + self.apples as u64 + self.pears as u64
    }
}
```

Read the `quote!` call as the output with holes in it.

- `#name` is replaced by the tokens of `name`. Here that is the user's own
  `Stock` token, with its span.
- `#( ... )*` is a repetition, as in `macro_rules!`. The part inside is
  written once per element of the iterator that is interpolated in it. Here
  each element produces `+ self.FIELD as u64`. A separator may stand before
  the `*`: `#(#a),*` writes commas between the elements.
- Everything else is copied as it stands.

What a value turns into depends on its type. An `Ident` becomes a name. A
`String` or `&str` becomes a *string literal*, with quotes. A number becomes a
literal with a type suffix: a `usize` 3 becomes `3usize`. A syn node becomes
its tokens. `Option<T>` becomes the tokens of the value, or nothing.

To make a *new* name, use `format_ident!("get_{}", field)`. It works like
`format!`, and it drops a leading `r#` from an identifier argument.

### Spans, and why you interpolate

The `Stock` and `apples` tokens in the output above are the very tokens that
the user wrote. They kept their spans. The tokens that `quote!` made new
(`impl`, `pub`, `fn`, `total`, `0`, `+`, ...) have the span of the macro call:
the word `Total` in `#[derive(Total)]`.

That is why lesson 01's shortcut (print the output to a string, then parse
the string) is a bad habit once the output contains the user's tokens. It
works, and every token then has the span of the macro call. The compiler and
the editor can no longer connect the generated code to what the user wrote.
Raw identifiers and unusual tokens are easy to break on the way through text,
too. Interpolate tokens; do not print and re-parse them.

### Names in the output are looked up at the call site

The output is pasted into the user's module and compiled there. A name such
as `Vec` in your output means whatever `Vec` means *in that module*. Usually
that is the standard `Vec`. It is not when the user has their own type of
that name. So write library names as full paths that start with `::`:
`::std::vec::Vec`, `::core::option::Option`, `::core::mem::replace`. The
leading `::` says "start at the list of crates", and nothing in the user's
module can change that. The same holds for the trait you implement:
`::m5_proc_macros::Changes`. Lesson 07 looks at the limits of this rule.

### Mistakes people really make

**A string where a name is needed.** If you collect field names as `String`
and interpolate them, `self.#names` becomes `self."apples"`:

```text
error: unexpected token: `"id"`
 --> use/src/main.rs:1:29
  |
1 | use mac::StrField; #[derive(StrField)] struct Job { id: u32, size: u32 } fn main(){}
  |                             ^^^^^^^^
...
error: proc-macro derive produced unparsable tokens
```

Keep the `Ident` for code, and make a string from it only for text.

**A `usize` as a tuple index.** For a tuple struct you need `self.0`, and an
interpolated `usize` prints as `0usize`:

```text
error: suffixes on a tuple index are invalid
 --> use/src/main.rs:1:29
  |
1 | use mac::TupleIdx; #[derive(TupleIdx)] struct Pair(u8, u8); fn main(){}
  |                             ^^^^^^^^ invalid suffix `usize`
```

Use `syn::Index::from(i)`, which prints as a bare number, or
`proc_macro2::Literal::usize_unsuffixed(i)`.

**Tests that compare printed tokens.** `tokens.to_string()` puts spaces
between tokens in a way that is not promised to stay the same
(`Vec < u8 >`). A test that compares such a string with a text you typed
breaks for reasons that have nothing to do with your macro. Parse the output
back into a syntax tree and look at its parts, or compare text with all white
space removed, as the helper `compact` in `macros/src/support.rs` does.

### Read more

- [Procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html) in the Rust Reference
- [Paths](https://doc.rust-lang.org/reference/paths.html) in the Rust Reference: what a leading `::` means
- [Derive macros](https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros) in the Rust Reference
- The documentation of the pinned crates, built from the exact versions in `Cargo.lock`: run `cargo doc -p syn -p quote -p proc-macro2 --open` in this package

## Lesson 03 · Generics, where clauses and helper attributes

**Assumed:** lesson 02, and generic parameters, bounds and where clauses from
Chapters 11 and 13.

### The mental model

Lesson 02 wrote `impl Trait for #name`. For `struct Wrapper<T>` that does not
compile: the type is `Wrapper<T>`, and `T` must be declared. An `impl` for a
generic type has three places where generics appear, and each wants a
different form of the same list:

```text
impl<'a, T: Clone, const N: usize> Trait for View<'a, T, N> where T: Send { .. }
    └──────────── 1 ─────────────┘              └─── 2 ───┘ └──── 3 ────┘
1  parameters with their bounds, without defaults
2  only the names
3  the where clause
```

`syn` gives you all three with one call:

```text
let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
quote! { impl #impl_generics Trait for #name #ty_generics #where_clause { .. } }
```

Each piece prints nothing when there is nothing to print, so the same line
works for a struct without generics. A default such as `T = u8` is allowed in
the declaration of a type and nowhere in an `impl`; `impl_generics` leaves it
out.

### Which bounds does the impl need?

The generated code *uses* the fields. If it calls `total()` on a field of
type `T`, the impl compiles only when `T: Total` is known. The derive has to
add bounds, and it must decide which ones while seeing syntax only. There are
three strategies.

1. **One bound per type parameter**: `T: Total` for every `T`. The standard
   derives do this. It is simple and predictable, and it is sometimes wrong
   in both directions. It is too strict when a parameter is not used in a way
   that needs the trait: `#[derive(Clone)] struct Shared<T>(Rc<T>)` is `Clone`
   only for `T: Clone`, although cloning an `Rc<T>` never clones a `T`
   (drill d01). And it does not see what is really used: a field of type
   `T::Item` needs a bound on `T::Item`, and a macro that adds only
   `T: Total` generates code that does not compile for such a struct. (The
   standard derives look for this one case. When a field's type names an
   associated type of a parameter, they add a bound such as `T::Item: Clone`
   next to `T: Clone`.)
2. **One bound per field type**: `FieldType: Total` in the where clause, for
   each field that the generated code uses. This asks for exactly what the
   code needs. It has two costs. The bounds show the types of private fields
   in the public signature of the impl. And it fails for a type that contains
   itself: with a field `next: Option<Box<Node<T>>>` and the derived trait as
   the bound, the bound `Option<Box<Node<T>>>: Total` asks for the very impl
   that it stands on. The compiler does not accept that circle. The impl
   compiles, and `Node<u8>` still does not implement the trait.
3. **No bounds**, or bounds that the user writes in an attribute. The user
   knows the types; the macro does not.

The worked example uses strategy 1, and the lesson uses strategy 2, so you
see both.

### Helper attributes

A derive may declare attributes of its own:

```text
#[proc_macro_derive(Total, attributes(total))]
```

Inside an item that derives `Total`, the compiler now accepts
`#[total(...)]` on the item, its fields and its variants. Such a *helper
attribute* does nothing by itself. It stays in the tokens, and the derive
finds it in `attrs` of the node it is written on. The tokens between the
parentheses are yours to define. Most macros use the form of a list of
`name` and `name = value` entries, and `syn` has a parser for exactly that:
`Attribute::parse_nested_meta`.

A node's `attrs` hold *all* its attributes: doc comments, `#[allow]`, the
helpers of other derives. Check the name first, and ignore what is not yours.

### A worked example

`Total` becomes a trait, and the derive supports generics and one helper:
`#[total(weight = 12)]` multiplies a field. For this input

```rust,ignore
struct CrateOf<T: Clone = u32>
where
    T: Send,
{
    #[total(weight = 12)]
    boxes: T,
    loose: T,
}
```

the derive below returns

```rust,ignore
impl<T: Clone + ::kit::Total> ::kit::Total for CrateOf<T>
where
    T: Send,
{
    fn total(&self) -> u64 {
        0 + 12 * ::kit::Total::total(&self.boxes) + 1 * ::kit::Total::total(&self.loose)
    }
}
```

```rust,ignore
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
```

Things to notice:

- `parse_nested_meta` calls the closure once per entry. `meta.path` is the
  entry's name. `meta.value()?` consumes the `=` and returns a parser for
  what follows it; `.parse()?` then reads a value of the type that the
  context asks for, here a `LitInt`. `meta.error(..)` makes an error that
  points at the entry.
- `syn::parse_quote!` is `quote!` plus a parse: it builds any syntax tree
  node from tokens, and the type comes from the context. Here it makes a
  literal and a bound.
- The clone of `generics` gets the new bounds, and `split_for_impl` is called
  on the clone. In the output, `T` has lost its default, kept its own bound
  and gained one, and the where clause is unchanged.
- `::kit::Total::total(&self.boxes)` names the trait in full instead of
  writing `self.boxes.total()`. A method call is looked up among all traits
  that are in scope at the call site, and the user's module may have another
  method called `total`.

### New tokens and their span

The bound `::kit::Total` above was made by the macro, so it has the span of
the derive. When a type does not satisfy a bound that your macro wrote, the
compiler's error points at that bound, which means: at `#[derive(Total)]`.
The user then has to guess which field is meant.

`quote_spanned!` is `quote!` with a span for the new tokens:

```text
quote_spanned! {ty.span()=> #ty: ::core::fmt::Debug }
```

Every token that this call *makes* gets the span of `ty`; interpolated tokens
keep their own. `syn::spanned::Spanned::span` gives the span of any syntax
tree node. With that, the error from the lesson's own compile-fail test
underlines the field's type:

```text
error[E0277]: `Opaque` doesn't implement `Debug`
  --> tests/ui/l03/fail_not_debug.rs:13:13
   |
13 |     handle: Opaque,
   |             ^^^^^^ the trait `Debug` is not implemented for `Opaque`
```

### Mistakes people really make

**No generics on the impl.**

```text
error[E0107]: missing generics for struct `Wrapper`
 --> use/src/main.rs:1:51
  |
1 | use mac::NoGenerics; #[derive(NoGenerics)] struct Wrapper<T> { inner: T } fn main(){}
  |                                                   ^^^^^^^ expected 1 generic argument
```

**The declaration's generics copied into the impl.** Writing
`impl #generics Trait for #name #generics` works until a parameter has a
bound or a default:

```text
error: expected one of `(`, `+`, `,`, `::`, `<`, or `>`, found `=`
 --> use/src/main.rs:1:70
  |
1 | use mac::RawGenerics; #[derive(RawGenerics)] struct Wrapper<T: Clone = u8> { inner: T } fn main(){}
  |                                                                      ^ expected one of `(`, `+`, `,`, `::`, `<`, or `>`
```

**The helper attribute is not declared.** Without `attributes(total)` in the
export line, the compiler does not know the attribute:

```text
error: cannot find attribute `total` in this scope
 --> use/src/main.rs:1:55
  |
1 | use mac::NoHelper; #[derive(NoHelper)] struct Job { #[total(skip)] id: u32 } fn main(){}
  |                                                       ^^^^^
```

**The struct's own where clause is lost.** If you build a new where clause
for your bounds, start from a copy of the struct's clause. `split_for_impl`
returns the struct's clause as it is; it does not know about bounds that you
add elsewhere.

### When to use which

Use per-parameter bounds when your trait is implemented for the usual
containers, so that `Vec<T>: Trait` follows from `T: Trait`, and when the
simple rule matters more to your users than the last bit of precision. Use
per-field-type bounds when the generated code touches only some fields, as in
the lesson, or when fields have types like `T::Item`. When neither is right
for a type, let the user state the bounds in an attribute.

### Read more

- [Generic parameters](https://doc.rust-lang.org/reference/items/generics.html) and [where clauses](https://doc.rust-lang.org/reference/items/generics.html#where-clauses) in the Rust Reference
- [Implementations](https://doc.rust-lang.org/reference/items/implementations.html) in the Rust Reference
- [Derive macro helper attributes](https://doc.rust-lang.org/reference/procedural-macros.html#derive-macro-helper-attributes) in the Rust Reference
- [Derive](https://doc.rust-lang.org/reference/attributes/derive.html) in the Rust Reference

## Lesson 04 · Errors that point at the right place, and a compile-fail suite

**Assumed:** lessons 02 and 03, and `Result` with `?` from Chapter 7.

### The mental model

A macro has only one way to tell the user about a mistake: it returns tokens
that make the compiler print an error. Lesson 01 built those tokens by hand.
`syn::Error` does it for you. A `syn::Error` holds a message and a span, and
`into_compile_error()` turns it into the tokens of
`::core::compile_error! { "message" }`, each carrying that span. The compiler
then prints the message under the code that the span covers.

So a good error is a good *span*. There are three ways to choose one:

- `syn::Error::new(span, message)`: an explicit span, for example
  `ident.span()`;
- `syn::Error::new_spanned(node, message)`: from the first to the last token
  of any syntax tree node or token stream. Use it when the wrong thing is
  more than one token, such as a type or a whole attribute;
- `meta.error(message)` inside `parse_nested_meta`, and `input.error(message)`
  inside a parser: the current position.

`Span::call_site()` is the span of the macro call, the word in
`#[derive(...)]`. An error with that span is the least helpful one you can
give. The two messages below have the same text; only the span differs.

```text
error: floating-point fields cannot be totalled
 --> use/src/main.rs:1:32
  |
1 | use mac::CallSiteErr; #[derive(CallSiteErr)] struct Reading { count: u32, ratio: f64 } fn main(){}
  |                                ^^^^^^^^^^^

error: floating-point fields cannot be totalled
 --> use/src/main.rs:1:80
  |
1 | use mac::SpannedErr; #[derive(SpannedErr)] struct Reading { count: u32, ratio: f64 } fn main(){}
  |                                                                                ^^^
```

### `syn::Result` and `?`

Functions inside a macro return `syn::Result<T>`, which is
`Result<T, syn::Error>`. Parsers return it too, so `?` carries a parse error,
with the span that syn gave it, up to the entry point. Only there does the
error become tokens. An entry point never panics and never returns `Err`: its
return type is `TokenStream`.

The macro `parse_macro_input!` from lesson 02 does the same for the first
step: when the input does not parse, it returns the error's tokens from the
exported function.

### Report every problem, not the first one

A user with three mistakes in one enum wants three errors from one compiler
run. `?` stops at the first. To collect, keep an `Option<syn::Error>` and
*combine*: `first.combine(second)` adds the second error to the first.
A combined error becomes one `compile_error!` call per problem, each with its
own span, and you can iterate over it to get the single errors back.

### A worked example

The derive `Total` from lesson 03 gets a check: a field of type `f32` or
`f64` is an error, because adding floats to a `u64` total would silently lose
their fractions. For this input

```rust,ignore
struct Reading {
    count: u32,
    ratio: f64,
    samples: Vec<u8>,
    drift: f32,
}
```

the check reports two errors, one under `f64` and one under `f32`:

```rust,ignore
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
```

`total_derive` is the shape of every entry point from here on: parse, check,
generate, with `?` between the steps, and one `match` at the end that turns
`Err` into tokens. Nothing in it can panic.

The check also shows a limit. It looks for a type *written* as `f64`. A type
alias `type Ratio = f64;` passes, because a macro sees the name `Ratio` and
not what it stands for. A macro's checks are checks of syntax.

### Testing what must not compile

A unit test can call `reject_floats` and look at the error: its message, and,
when `proc_macro2` is built with its `span-locations` feature, the line and
column of its span. The unit tests in this module do that. They are fast and
exact, and they cannot answer the last question: what does the *compiler*
print for a real user?

The `trybuild` crate answers it. A test gives it a file that must not
compile:

```text
#[test]
fn errors() {
    trybuild::TestCases::new().compile_fail("tests/ui/l04/fail_*.rs");
}
```

For each file, trybuild builds a small crate that depends on yours, compiles
the file, and compares the compiler's error output with the file of the same
name ending in `.stderr`. The test fails when the file compiles, or when the
output differs in a single character. So a `.stderr` file records messages
*and* spans: the line numbers, the columns, and the `^^^` underlines are all
in it. `pass` is the opposite: the file must compile, and its `main` must run
without a panic.

The work cycle:

1. Write the failing program, `tests/ui/l04/own_case.rs`.
2. Run the test with `TRYBUILD=overwrite` set. trybuild writes the compiler's
   output to `own_case.stderr`. Without the variable, it writes the output to
   a directory `wip/` and fails the test, so that nothing becomes "expected"
   by accident.
3. **Read the `.stderr` file.** It is now the specification. If a message
   points at the derive although one field is wrong, the file records a bug.
4. Commit both files. From now on the test fails when the output changes.

trybuild replaces things that differ between machines, such as the path to
the standard library (`$RUST`), so the files are portable. They are still
tied to one compiler version: a new compiler may word an error differently.
That is why this course pins the toolchain, and why real projects run their
compile-fail suite on one fixed toolchain only.

### Mistakes people really make

**`unwrap()` on a parse.** `attr.parse_args::<LitStr>().unwrap()` turns the
user's typing mistake into "proc-macro derive panicked" at the derive
(lesson 01). The `Err` already has a good message and a good span: pass it on
with `?`.

**Returning at the first error.** The user fixes one mistake, compiles, and
gets the next. Collect and combine.

**Accepting a `.stderr` file without reading it.** `TRYBUILD=overwrite` makes
every test green. It records what the macro does, which is only useful when a
person has checked that this is what it should do.

**Recording compiler errors you do not control.** A `.stderr` file with a
type error from the compiler, about code that your macro generated, depends on
details of the generated code and on the compiler's wording. Keep such cases
few, and prefer cases whose errors your macro writes itself.

### Read more

- [`compile_error!`](https://doc.rust-lang.org/core/macro.compile_error.html)
- [Procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html) in the Rust Reference: the two ways in which a macro can report an error
- [`proc_macro::Span`](https://doc.rust-lang.org/proc_macro/struct.Span.html)
- [The `?` operator](https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator) in the Rust Reference
- `cargo doc -p syn -p trybuild --open` in this package, for `syn::Error` and for trybuild's own guide

## Lesson 05 · An attribute macro that keeps the item intact

**Assumed:** lessons 02 to 04, and `Drop` from Chapter 13.

### The mental model

An attribute macro is exported like this:

```text
#[proc_macro_attribute]
pub fn logged(args: TokenStream, item: TokenStream) -> TokenStream
```

For `#[logged(label = "db")] fn load() { .. }`, `args` holds the tokens
between the parentheses of the attribute (`label = "db"`; empty for a plain
`#[logged]`), and `item` holds the item below it, with all its other
attributes. The returned tokens **replace** the item. This is the difference
from a derive: a derive adds code, an attribute macro is responsible for the
item itself. If it returns nothing, the function is gone.

That responsibility shapes how you write one. The safe method has three
steps:

1. parse the item into its syntax tree (`syn::ItemFn` for a function; this
   needs syn's `full` feature);
2. change the one part that you mean to change;
3. print the whole tree again.

The other method, taking the function apart and writing a new one with
`quote!`, has to mention every part: attributes, visibility, `const`,
`async`, `unsafe`, the ABI, generics, parameters, the return type, the where
clause. The part you forget is silently dropped, and the macro works until a
user writes an `async fn` with a where clause.

### A worked example

`#[rename(new_name)]` gives a function another name. Its argument is one
identifier:

```rust,ignore
/// `#[rename(new_name)]`: the same function under another name.
pub(crate) fn rename(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let new_name: Ident = syn::parse2(args)?;
    let mut function: ItemFn = syn::parse2(item)?;
    function.sig.ident = new_name;
    Ok(function.into_token_stream())
}
```

`syn::parse2::<Ident>(args)` reads the argument and fails, with a good
message, on anything else: no argument, two arguments, a string. An `Ident`
is one of many types that implement syn's `Parse` trait. When the arguments
have a shape of their own, as in the lesson, you write a small type and
implement `Parse` for it; lesson 06 goes deep into that.

`function.sig` is the signature: everything from `const`/`async`/`unsafe` and
`fn` through the name, the generics, the parameters and the return type, to
the where clause. The body is `function.block`, a list of statements,
`function.block.stmts`. The attributes and the visibility are separate fields.

The example returns `syn::Result`. Its entry point has to turn an `Err` into
tokens, and here an attribute macro has a duty that a derive does not have:
**on an error, return the item as well.**

### Why the item must survive an error

Suppose the argument is wrong and the macro returns only its
`compile_error!`. The function no longer exists. The compiler prints the
macro's error, and it is careful: it holds back "cannot find function"
errors, because it knows that a macro failed and that items may be missing.
It also cannot check any code that *uses* the function. So mistakes in that
code stay hidden until the argument is fixed, and the user needs one more
round.

If the macro returns the error followed by the unchanged item, the function
exists, and the compiler checks its uses in the same run. Tools that keep
working on code with errors, such as the language server of an editor, are in
the same position: they see the macro's output, and nothing else.

The lesson's compile-fail test records exactly this. After the macro's three
errors, the compiler finds a fourth, in `main`, that it could not have found
without the functions:

```text
error: expected string literal
  --> tests/ui/l05/fail_args.rs:21:18
   |
21 | #[logged(label = 7)]
   |                  ^

error[E0308]: mismatched types
  --> tests/ui/l05/fail_args.rs:28:25
   |
28 |     let _text: String = total;
   |                ------   ^^^^^ expected `String`, found `u32`
```

### Doing something at the end of a function

The lesson's macro has to write a line into a log when the function starts
and another when it ends. "When it ends" is harder than it sounds. A function
ends at its last expression, at any `return`, at any `?` that sees an error,
and when it panics. Code that the macro appends after the body runs in the
first case only.

There are two general techniques.

- **Wrap the body** in a closure or an inner function, call it, and do the
  work after the call. This catches `return` and `?`. It changes what the
  body means in other ways: a closure captures variables instead of using
  parameters directly, an `async` body needs an `async` wrapper, and a panic
  still skips your code.
- **A guard.** Put one `let` at the top of the body that creates a value
  whose `Drop` does the work. Rust drops local variables when the function
  ends, in every one of the four cases. The body itself is not touched.

The lesson uses a guard. Two details matter. The guard must be bound to a
named variable: `let _ = make_guard();` drops the value at once, because `_`
is not a variable. And the name is now a local variable in the *user's*
function, so it should be a name that the user will not use. Lesson 07
explains what can go wrong and how to rule it out; for now, a name such as
`__logged_guard` is enough.

### Which attributes does the macro see?

`item` contains the attributes above and below the macro's own attribute,
except the macro's own. A listing from a test macro, for an attribute macro
`show` and a derive:

```text
/// Doc.                              args: [label = "x", 3]
#[inline]                             item: /// Doc.
#[show(label = "x", 3)]                     #[inline] #[must_use] pub fn f() -> u8 { 1 }
#[must_use]
pub fn f() -> u8 { 1 }
```

Attribute macros on one item are expanded from top to bottom, and each one
sees the output of those above it. `#[derive(...)]` attributes follow the same
order. A derive sees the item without the derive attribute that names it and
without the derive attributes above that one, which have been expanded
already. A derive attribute further down is still in the tokens:

```text
#[derive(Show)]                       Show receives:
#[derive(Debug)]                          #[derive(Debug)] #[allow(dead_code)] struct A;
#[allow(dead_code)]
struct A;
```

### Mistakes people really make

**Rebuilding the function from parts.** Covered above: a forgotten part is
dropped without any error. Keep the tree and change one field.

**`let _ = guard;`** The exit line is written before the body runs. The unit
test of the lesson checks the pattern, and the run-time test checks the
order of the log lines.

**Using the macro where it cannot work.** A `const fn` cannot call a
function that is not `const`. If the macro adds such a call anyway, the user
gets two errors about generated code, both at the attribute. A macro that
knows its limits says so itself, at the keyword:

```text
error: `#[logged]` cannot be used on a `const fn`
  --> tests/ui/l05/fail_wrong_item.rs:14:1
   |
14 | const fn limit() -> u32 {
   | ^^^^^
```

### When to use which

Use a derive when you add an impl for a type and leave the type alone. Use an
attribute macro when you must change an item, or when the item is not a
struct, enum or union. Because an attribute macro can rewrite anything, it is
also the kind that surprises readers most: what they see in the source is not
what is compiled. Keep the change small and say in the documentation what it
is.

### Read more

- [Attribute macros](https://doc.rust-lang.org/reference/procedural-macros.html#attribute-macros) in the Rust Reference
- [Functions](https://doc.rust-lang.org/reference/items/functions.html) in the Rust Reference: every part of a function item
- [Destructors](https://doc.rust-lang.org/reference/destructors.html) in the Rust Reference: when local variables are dropped
- [Wildcard pattern](https://doc.rust-lang.org/reference/patterns.html#wildcard-pattern) in the Rust Reference: why `let _ =` does not keep a value

## Lesson 06 · A function-like macro with its own syntax

**Assumed:** lessons 02 to 05.

### The mental model

`name!(...)` is the freest kind of macro. The input is whatever tokens the
user wrote between the delimiters; the output replaces the whole call. The
input does not have to be Rust. It only has to be *tokens*: literals must be
well formed, and brackets must be balanced, because the tokenizer has already
built the token trees before your macro runs. Inside those limits you design
a small language.

Where the call stands decides what the output may be. A call at the top
level of a module must expand to items. A call in an expression must expand
to an expression:

```text
error: expected expression, found keyword `struct`
 --> use/src/main.rs:1:41
  |
1 | use mac::item_macro; fn main(){ let x = item_macro!(); }
  |                                         ^^^^^^^^^^^^^ expected expression
```

A language of your own needs a parser of your own. So far `syn` parsed whole
Rust items for you. Its parsing machinery is public, and writing a parser
with it is the skill of this lesson.

### Parsers and the cursor

A syn parser is a function from a `ParseStream` to a `syn::Result<T>`. A
`ParseStream` is a position in a token stream, a *cursor*, that moves forward
as tokens are consumed. The main operations:

| Call | What it does |
| --- | --- |
| `input.parse::<T>()?` | parses a `T` at the cursor and moves past it; an error leaves a message at the token that did not fit |
| `input.parse::<Token![=]>()?` | the same for one punctuation mark or keyword |
| `input.peek(Token![,])` | looks at the next token without moving |
| `input.is_empty()` | true at the end of the stream |
| `input.call(f)?` | runs another parser function at the cursor |
| `braced!(content in input)` | takes the next `{ ... }` group and gives a new stream for its inside; `parenthesized!` and `bracketed!` do the same for the other delimiters |
| `content.parse_terminated(T::parse, Token![,])?` | a list of `T` separated by commas, with an optional last comma, into a `Punctuated<T, Token![,]>` |
| `input.error("message")` | an error at the cursor |

A type gets a parser by implementing the trait `syn::parse::Parse`. Every
syntax tree type of syn implements it, and so can yours. Then `input.parse()`
works for your type, and the types compose: a list of entries is
`Punctuated<Entry, Token![,]>`.

A parser must consume the *whole* input. `syn::parse2` and
`Parser::parse2` check this: when your parser returns `Ok` and tokens are
left, the result is the error `unexpected token` at the first token that was
left over. This rule catches a whole class of mistakes for you, and it is the
reason that `syn::parse_str::<Type>("u8 extra")` fails.

### A worked example

`colors!` declares named colours:
`colors! { RED = "#ff0000", GREEN = "#00ff00" }` expands to one `u32` constant
per entry.

```rust,ignore
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
```

The parser for one entry reads top to bottom like the grammar: a name, `=`, a
string. Each `?` is a place where the input may be wrong, and the error that
syn writes there (`expected identifier`, ``expected `=` ``,
`expected string literal`) already points at the token.

The second half of `Color::parse` is a different kind of check. The tokens
have the right shape, but the *content* of the string is wrong. syn cannot
know that; you write the error and choose its span, here the literal. A
macro that validates at compile time what would otherwise fail at run time is
one of the best uses of the function-like kind.

`colors` parses the list with `Parser::parse2` on the function
`Punctuated::parse_terminated`. That is how you run a parser *function* on a
token stream when there is no type to implement `Parse` for.

### Parsing first, checking after

`Color::parse` returns at its first error, and for one entry that is right:
after a token that does not fit, a parser does not know where it is. Checks
that need the whole input come later, on the parsed data, and they should
report everything they find, as in lesson 04. The lesson's macro has three
such checks. A useful structure is three functions:

```text
tokens ──parse──▶ your types ──check──▶ your types, known to be valid ──generate──▶ tokens
        syn's errors,          your errors, all of them,
        first one              each with a span
```

Keep spans in your types. An entry that stores its name as an `Ident`, and
not as a `String`, can still say where it was written when a later check
finds it twice.

### Generated items and constants

A function-like macro at module level creates items in the user's module, so
everything from lesson 02 about paths applies, and a little more. The output
of `colors!` has `u32` in it. A module can declare its own type named `u32`;
primitive types are ordinary names that can be shadowed. The full path is
`::core::primitive::u32`. For built-in derives there are paths too:
`#[derive(::core::clone::Clone, ::core::fmt::Debug)]`.

If the user should be able to use the generated functions in a `const` or a
`static`, they must be `const fn`. A macro cannot find this out by trying; you
decide it when you design the output, and a test that uses the function in a
`const` keeps it true.

### Mistakes people really make

**Forgetting the last comma.** `A, B,` and `A, B` should both be accepted.
`parse_terminated` does that; a hand-written loop often does not.

**Peeking without consuming.** A loop `while input.peek(Token![,]) { ... }`
whose body forgets to parse the comma never ends. A macro that does not
return hangs the user's compiler. Every pass through a loop must move the
cursor.

**Errors at the wrong level.** `input.error(..)` inside the braces of a
`braced!` group points into the group; the same call on the outer stream
points after the group. Make the error on the stream that holds the token you
mean, or use the span of a token you have parsed.

**Reporting a duplicate without a place.** "duplicate flag `READ`" with the
span of the macro call makes the user search. Point at the second occurrence.

### When to use which

Choose a function-like macro when the input is not a Rust item: a small
language, a table, a literal that is checked while compiling. When the input
*is* an item with a few extras, an attribute macro on a real item is kinder
to editors and to readers, because the item stays valid Rust. And when
`macro_rules!` can express the input, prefer it: it needs no extra crate and
no compile time for `syn`. Reach for a procedural macro when you need to
compute (count, check, look for duplicates) or need error messages of your
own.

### Read more

- [Function-like procedural macros](https://doc.rust-lang.org/reference/procedural-macros.html#function-like-procedural-macros) in the Rust Reference
- [Macro invocations](https://doc.rust-lang.org/reference/macros.html#macro-invocation) in the Rust Reference: the places where a macro call may stand
- [Constant evaluation](https://doc.rust-lang.org/reference/const_eval.html) in the Rust Reference: what a `const fn` may do
- [`core::primitive`](https://doc.rust-lang.org/core/primitive/index.html)
- `cargo doc -p syn --open` in this package: the module `syn::parse` has a guide to writing parsers

## Lesson 07 · Hygiene, crate paths, and how to test a macro

**Assumed:** lessons 02 to 06, and hygiene of `macro_rules!` from Chapter 22.

### The mental model

Generated code is written by you and compiled in somebody else's module.
Every name in it is looked up *somewhere*, and "hygiene" is the set of rules
that say where. A macro is hygienic when its expansion means what you meant,
whatever names the user's module contains, and when it does not change what
the user's code means.

In a procedural macro, the rule is carried by the span. A token's span says
where the token came from for error messages, and it also says in which scope
the token's name is resolved. There are three spans to know:

| Span | A name with this span is resolved... |
| --- | --- |
| the user's own tokens | where the user wrote them |
| `Span::call_site()` | as if the user had written the token at the macro call. This is the span of everything that `quote!` creates. |
| `Span::mixed_site()` | like `macro_rules!`: a *local variable* or a loop label belongs to the macro alone; every other name is resolved at the call site. The `self` of a method counts as a local variable here. |

(`Span::def_site()`, "inside the macro crate", is not available on the stable
compiler.)

There are three kinds of names in an expansion, and each has its own danger
and its own protection.

### 1. Local variables

`timed!(expr)` should give the value of `expr` and the time it took:

```rust,ignore
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
```

Everything `quote!` makes has the span `call_site`, so `start` and `value`
are variables in the user's function, as if the user had declared them. Now a
user writes this:

```text
let start = 10;
let (sum, _took) = timed!(start + 5);
assert_eq!(sum, 15);
```

The expansion's `let start = Instant::now();` comes before `start + 5` and
shadows the user's variable. The expression now adds 5 to an `Instant`, which
gives an `Instant`, not a number:

```text
error[E0308]: mismatched types
 --> use/src/main.rs:1:98
  |
1 | use mac::timed; fn main(){ let start = 10; let (sum, _took) = timed!(start + 5); assert_eq!(sum, 15); }
  |                                                                                                  ^^ expected `Instant`, found integer
```

Here the types differ, so the user gets an error, a confusing one. When the
types happen to agree, the program compiles and computes with the wrong
value. That is the worst outcome a macro can produce.

The protection is `mixed_site`:

```rust,ignore
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
```

`quote_spanned!` gives every token that it *makes* the span `mixed_site`. The
interpolated `#expr` keeps the user's spans. Now `__timed_start` is a variable
that only the macro's own tokens can name. The user's `start`, and even a
user variable called `__timed_start`, are different variables, and neither
side sees the other.

### 2. Names of items, and why the names start with `__`

`mixed_site` protects *variables*. It does nothing for items: a path such as
`Instant`, `Option` or `Ok` in your expansion is resolved at the call site,
with any span. The protection for paths is the one from lesson 02: write
them in full, from `::std` or `::core`.

There is a place where the two meet, and it is easy to miss. A plain name in
a *pattern* is not always a new variable. `let limit = ..` declares a
variable only when no item named `limit` is in the way. If a constant, a
unit struct or an enum variant of that name is in scope, the name means that
item, and no variable is made. If a static or a tuple struct has the name,
the compiler rejects the `let`. Function parameters and `match` arms are
patterns too. Hygiene does not help here: the question "is there an item of
this name?" is asked at the call site.

So an expansion with `let value = ..` breaks in a module that has
`struct value;`, with `mixed_site` or without. Such names go against Rust's
naming conventions and are rare, and a library macro should still survive
them. The protection is a name that nobody else uses: generated variables and
parameters start with `__` and the macro's name, such as `__timed_start`.
This is why the model answers of this module write `__value` and `__newer`
from lesson 02 on.

Method calls are a third way to meet the user's names. `x.clone()` is looked
up among the traits in scope at the call site. Write
`::core::clone::Clone::clone(&x)` when it matters which `clone` is called.

### 3. The path to your own crate

A derive implements a trait, and it must name that trait:
`::m5_proc_macros::Named`. A `macro_rules!` macro has `$crate` for this. A
procedural macro has nothing of the kind, and the absolute path has two
weaknesses.

- **Inside the crate itself.** `::m5_proc_macros` means "the crate that is
  called `m5_proc_macros` here". Within `m5_proc_macros` there is no such
  crate, only `crate`. The fix is one line in the library,
  `extern crate self as m5_proc_macros;`, which gives the crate that name for
  its own code. Open `src/lib.rs` of this package: the line is there.
- **Renamed dependencies.** A user may write
  `kit = { package = "m5_proc_macros", .. }` in their `Cargo.toml`, or get
  your crate only through another crate that re-exports it. Then
  `::m5_proc_macros` does not exist for them. The usual fix is an argument
  that tells the derive where the crate is: `#[named(crate = ::kit)]`. The
  derive uses that path instead of its default.

### What each kind of test can show

You now have four kinds of test. Each proves something that the others
cannot.

| Test | Proves | Cannot see |
| --- | --- | --- |
| Unit test on tokens (`proc_macro2`, in the macro crate) | the shape of the output: which items, which paths, which bounds; error messages and their positions; that nothing panics | whether the output compiles, and anything about hygiene |
| `trybuild` pass case | the output compiles and behaves correctly at run time | why something fails; how an error looks |
| `trybuild` compile-fail case | exactly what a user sees for a mistake | anything about valid input |
| A pass case in a *hostile* module | hygiene: the expansion survives a module with its own `Option`, `Vec`, `value`, ... | only the names you thought of |

Hygiene needs the last kind. Two expansions that differ only in the span of
one identifier print the same text, so no test on tokens can tell them apart:
`Ident`'s `==` compares the letters and ignores the span. The compiler is the
only judge.

When you write checks on tokens, check structure, not text. Parse the output
with syn and look at the trait path, the generics, the where clause. Such a
test says what is wrong when it fails, and it does not break when a new
version of a crate prints tokens with different spacing. Task C of the lesson
makes you write such checks for a derive, and runs them against five broken
implementations.

To *look* at an expansion while you debug, use the expansion view of your
editor, or the tool `cargo expand` when it is installed. It prints the
expanded code of a crate. Remember what it cannot show: spans.

### Mistakes people really make

**Trusting `call_site` because the tests pass.** Tests rarely use a variable
with the same name as the macro's own. Users do, sooner or later.

**Wrapping the user's expression in a closure to isolate it.** Inside a
closure, `?` and `return` in the user's expression leave the closure instead
of the user's function, and `break` no longer compiles. Hygiene gives you the
isolation without changing control flow.

**A loop around the user's expression.** An unlabelled `break` or `continue`
in the user's expression now means the macro's loop. No span changes that;
only labels are hygienic. If your expansion puts a user's expression inside a
loop, as the lesson's `attempt!` does, say so in the macro's documentation.

**`use` lines in the expansion.** `use ::std::fmt::Debug;` at module level
collides with the user's own import of the same name. Full paths at each use
cost a few more tokens and cannot collide.

### Read more

- [Procedural macro hygiene](https://doc.rust-lang.org/reference/procedural-macros.html#procedural-macro-hygiene) in the Rust Reference
- [`Span::mixed_site`](https://doc.rust-lang.org/proc_macro/struct.Span.html#method.mixed_site) and [`Span::call_site`](https://doc.rust-lang.org/proc_macro/struct.Span.html#method.call_site)
- [Macro hygiene](https://doc.rust-lang.org/reference/macros-by-example.html#hygiene) in the Rust Reference: the rules of `macro_rules!`, which `mixed_site` copies
- [Identifier patterns](https://doc.rust-lang.org/reference/patterns.html#identifier-patterns) in the Rust Reference: when a name in a pattern is not a new variable
- [Extern crate declarations](https://doc.rust-lang.org/reference/items/extern-crates.html) in the Rust Reference, for `extern crate self as name;`
- [Renaming dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml) in the Cargo book

## Lesson 08 · Capstone: `#[derive(Builder)]`

**Assumed:** everything above.

The capstone has no new idea. It asks you to hold all of them at once, in a
macro that is large enough to need a design. This section is a way to work,
not a solution.

**Write the expansion by hand first.** Take the example struct of the
contract and write, as ordinary Rust in a scratch file, the three items that
the derive must produce. Make that file compile and try it. Now you know the
target, and you will find the awkward spots (what is the type of a builder
field for an `Option<String>` field? how does `build` give all missing names
at once?) while the compiler still gives you ordinary error messages.

**Decide from syntax, and write the decisions down.** The macro treats a
field as optional when its type is *written* as a path that ends in
`Option<X>`. That is a decision about tokens. It is wrong for
`type Maybe<T> = Option<T>;` (not recognised: the field is required and of
type `Maybe<T>`), and for a user's own type that happens to be called
`Option` (recognised by mistake). Every derive that looks at types has this
limit. State it in the documentation of your macro, as the contract does.

**Three steps, one direction.** Parse the input into a list of plain values,
one per field, that hold everything the generator needs: the name, the type,
the type of the setter's value, what happens when the field was never set.
Check, and collect all errors. Generate from the list. When generation never
has to look at an attribute again, each step stays small enough to test.

**Find the place where the user's code runs.** `default = EXPR` puts an
expression of the user into a function body that you wrote. That is the
situation of lesson 07, part 1: your local variables and the user's
expression share a scope unless you separate them. The other generated
functions contain no user code, but their parameters are patterns in the
user's module (lesson 07, part 2).

**Give that function one span, `self` included.** For hygiene, the `self` of
a method is a local variable. A use of `self` finds the parameter only when
both tokens are resolved in the same way: both with `mixed_site`, or both at
the call site. If the signature comes from `quote!` and the body from
`quote_spanned!` with `mixed_site`, your own `self.field` no longer compiles:

```text
error[E0424]: cannot find value `self` in this scope
  --> tests/ui/l08/pass_hostile.rs:32:14
   |
32 |     #[derive(m5_proc_macros::Builder)]
   |              ^^^^^^^^^^^^^^^^^^^^^^^
   |              |
   |              `self` value is a keyword only available in methods with a `self` parameter
   |              this function has a `self` parameter, but a macro invocation can only access identifiers it receives from parameters
...
```

So write the whole function, signature and body, in one `quote_spanned!`.
The same rule then works for you: a user's `default = self.x` cannot reach
your builder, and `tests/ui/l08/fail_default_self.rs` checks that. Remember
that interpolated tokens keep their own span. A local name that you build
yourself needs the span too: `Ident::new("__required_0", Span::mixed_site())`.

**Order the tests as you ordered the work.** The unit tests in the lesson
file check the shape of the three items. `tests/ui/l08/pass_basic.rs` checks
behaviour, `pass_generic.rs` checks generics and visibility,
`pass_defaults.rs`, `pass_hostile.rs` and `fail_default_self.rs` check
hygiene, and the other `fail_*` cases check errors. When a pass case fails to compile, the compiler output that
trybuild prints is about *your generated code*: read it with the expansion
next to it.

### Read more

- [Visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) in the Rust Reference
- [Type aliases](https://doc.rust-lang.org/reference/items/type-aliases.html) in the Rust Reference
- [`let`-`else`](https://doc.rust-lang.org/reference/statements.html#let-statements) in the Rust Reference
- [The `missing_docs` lint](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#missing-docs) in the rustc book
