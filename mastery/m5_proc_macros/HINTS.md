# Mastery M5 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [All chapters](../../README.md)

Each lesson has two hints. Open the first one only after a real attempt, and the second one only if the first was not enough. The rules in each contract ("spanned on the fields", "must never panic") are part of the exercise. *Check your explanation* is a model answer to the lesson's EXPLAIN prompt; read it after you have written your own.

## 01 · A derive with nothing but `proc_macro`

<details><summary>Hint 1</summary>

Walk the tokens with `for token in stream` or with `stream.into_iter()` and `next()`, and `match` on the four kinds of `TokenTree`. For task A, attributes and `pub(...)` hide their contents inside groups, so at the top level the first identifier that reads `struct`, `enum` or `union` is the keyword, and the next token is the name. For task D, keep a counter for `<` and `>`, and remember the previous punctuation character so that the `>` of `->` is not counted.

</details>
<details><summary>Hint 2</summary>

For task B, make a `Vec<TokenTree>`: `Punct::new(':', Spacing::Joint)` and `Punct::new(':', Spacing::Alone)` form one `::`, then `Ident::new("core", span)`, the same again for `compile_error`, then `Punct::new('!', Spacing::Alone)`, a `Group::new(Delimiter::Parenthesis, ..)` around `Literal::string(message)`, and `;`. Call `set_span(span)` on every token, and on the literal too: setting the span of a group does not reach the tokens inside it. For tasks C and E, `format!("impl ::m5_proc_macros::TypeName for {name} {{ ... }}").parse()` is enough; `{text:?}` prints a string as a literal with quotes, and `{names:?}` prints a `Vec<String>` as `["a", "b"]`, which is a valid array expression. A field in task D starts with any number of `#` + group pairs, then perhaps `pub` and perhaps one group, then the name.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A derive macro is a function in a crate marked `proc-macro = true`. The compiler compiles that crate first and calls the function when it meets `#[derive(Name)]` in another crate. The function receives the tokens of the item, with its attributes and without the derive attribute that named the macro, and returns new tokens. The compiler adds those after the item; the item itself is not changed. The input is a sequence of token trees: identifiers, single punctuation characters, literals, and groups for `()`, `[]` and `{}`. Angle brackets are not groups, because the tokenizer cannot tell a type argument list from a comparison. So a comma inside `HashMap<String, u32>` is at the same level as the comma between two fields, and the macro has to count `<` and `>` itself. Tokens carry no grammar: finding out what they mean is parsing, and without a library the macro author writes that parser.

</details>

## 02 · `syn` and `quote` for structs with named fields

<details><summary>Hint 1</summary>

For task A, `match` on `input.data` and then on the `fields` of the struct; only `Fields::Named` has what you want. For tasks B and C, write the `quote!` call for one field first, in a closure that you `map` over the fields. Then interpolate the iterator of results into the outer `quote!` with `#(#checks)*`.

</details>
<details><summary>Hint 2</summary>

`field.ident` is an `Option<Ident>`; for a named field, `as_ref()` and `expect(..)` with a reason is fine. Interpolate that `Ident` for code (`self.#ident`). For the text, make a `String`: `syn::ext::IdentExt::unraw(ident).to_string()` drops `r#`, and a `String` interpolates as a string literal. In task C, `format_ident!("set_{}", ident)` builds the method name and drops `r#` by itself; `#vis` and `#ty` come straight from the field.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The compiler hands the derive the tokens of `struct Job { id: u32 }`. `parse_macro_input!` turns them into a `DeriveInput`: the name `Job`, empty generics, and `Data::Struct` with one named `Field` whose `ident` is `id` and whose `ty` is `u32`. The function builds the output with `quote!`. The tokens `Job`, `id` and `u32` in the output are the user's own tokens, taken from the tree and interpolated, and they keep the spans of the user's source. Everything else (`impl`, `fn`, the path of the trait, the string `"id"`) is new and has the span of the derive attribute. This matters because the compiler and the editor use spans to connect code to source text: an error or a "go to definition" on a reused token leads to the user's field, and one on a new token leads to the derive line. It is also why names in the output must be full paths: the output is resolved in the user's module, not in the macro crate.

</details>

## 03 · Generics, where clauses and helper attributes

<details><summary>Hint 1</summary>

For task A, loop over `field.attrs`, skip every attribute whose `path()` is not `redacted`, and call `parse_nested_meta` on the others; inside the closure, compare `meta.path` with `is_ident`. For task B, start from a clone of `generics.where_clause`, which is an `Option`. For task C, call `split_for_impl`, use its first two pieces, and replace the third by the result of task B.

</details>
<details><summary>Hint 2</summary>

In task A, `meta.value()?.parse::<syn::LitStr>()?` reads `= "text"`, and returning `Err(meta.error(..))` from the closure rejects an unknown argument; a `?` after `parse_nested_meta(..)` passes that on. In task B, `clause.get_or_insert_with(|| syn::parse_quote!(where))` gives you a where clause to push to, and `syn::parse_quote_spanned!(ty.span()=> #ty: ::core::fmt::Debug)` builds one predicate with the span of the type (`use syn::spanned::Spanned;`). Return `None` when the clause has no predicates. In task C, the macro knows at expansion time which fields are printed, so the separators `" "` and `", "` can be part of the string literals that it generates: nothing has to be decided at run time.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`split_for_impl` returns three pieces of one list of generics. The first goes after `impl`: every parameter with its bounds and without its default. The second goes after the type's name: only the names. The third is the where clause and goes before the body. A bound per type parameter (`T: Debug`) is simple, but it looks at the parameter list and not at the fields. For `struct Wrapper<T> { #[redacted(mask)] inner: T }` it is too strict: `T` is never printed, and the impl would still demand `T: Debug`. For `struct First<T: Iterator> { item: Option<T::Item> }` it asks for the wrong thing: the code prints a `T::Item`, and `T: Debug` says nothing about that. A bound per field type (`Option<T::Item>: Debug`) asks for exactly what the generated code uses. It fails for a type that contains itself when the bound names the derived trait, because the bound then needs the impl that it belongs to, and it shows the types of private fields in the impl's signature.

</details>

## 04 · Errors that point at the right place, and a compile-fail suite

<details><summary>Hint 1</summary>

Keep one `Option<syn::Error>` per function and add to it with `combine`; return `Err` at the end if it holds something. In task A, do not use `?` on `parse_args`, because that would skip the checks that follow; `match` on the result and collect the error. Go through the attributes first and look at the fields after that: this gives the source order that the contract asks for.

</details>
<details><summary>Hint 2</summary>

`syn::Error::new_spanned(node, "..")` takes anything that can be printed as tokens: an `Attribute`, `&variant.fields`, the keyword `data.struct_token`, a `LitStr`. For the default label, `LitStr::new(&name, variant.ident.span())` makes a literal that carries the variant's span, so that the duplicate check in task B can treat both kinds of label alike and call `new_spanned(&label, ..)`. In task B, compare `label.value()` with the labels found so far and remember the *first* variant for each text. In task C, `syn::parse2::<DeriveInput>(input)` returns a `Result`; turn each `Err` into tokens with `into_compile_error()` and return them. Write the body of `label` as `match *self { .. }`: for an enum without variants the arms are empty, and `match self {}` on a reference is rejected with E0004, because a reference always counts as a value that can exist. For task D, a union, or `#[label]` without parentheses, or a default label that collides with an explicit one, are all cases that the supplied files do not have.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A `syn::Error` holds one or more messages, each with a span. `into_compile_error()` turns it into tokens: one call of `::core::compile_error!` per message, with the message's span on the tokens of the call. The macro returns these tokens as its output, the compiler expands them, and it prints each message under the source text that the span covers. So the place where an error appears is decided completely by the span that the macro chose. A `.stderr` file holds the compiler's full output: the message, the file, line and column, the source line, and the `^^^` underline. If the macro changes a span from the field to the whole derive, the message stays the same and the file no longer matches. That makes it a test of spans. A unit test with `span-locations` checks the same thing faster, and the `.stderr` file checks it as the user sees it.

</details>

## 05 · An attribute macro that keeps the item intact

<details><summary>Hint 1</summary>

For task A, a loop `while !input.is_empty()` that parses a name (`Ident`), then `Token![=]`, then a `LitStr`, then a comma unless the input has ended. Make your own errors with `syn::Error::new(name.span(), ..)`. For task B, write a helper that returns `syn::Result<TokenStream>` and uses `?`, and let `logged` turn its `Err` into the `compile_error!` tokens *followed by the original item*. Clone the item's tokens before you parse them, so that you still have them on the error path.

</details>
<details><summary>Hint 2</summary>

In the helper: `syn::parse2::<LoggedArgs>(args)?`, then `syn::parse2::<ItemFn>(item)?`. Check `function.sig.constness` and make the error with `new_spanned` on that token. Build the new statement with `syn::parse_quote! { let #guard = ::m5_proc_macros::calls::enter(#label); }` and put it in with `function.block.stmts.insert(0, statement)`. Print the function with `function.into_token_stream()` (`use quote::ToTokens;`). The default label is `LitStr::new(&name, function.sig.ident.span())`, where `name` comes from `syn::ext::IdentExt::unraw`. For the guard's name, `Ident::new("__logged_guard", Span::mixed_site())` is the careful choice; lesson 07 explains the span.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An attribute macro receives two token streams: the tokens between the parentheses of the attribute, and the item under it with its other attributes. What the macro returns replaces the item, so the original function exists afterwards only if the macro puts it into its output. Parsing the function into an `ItemFn`, changing one field and printing the tree keeps every part that syn knows about: attributes, visibility, `async`, generics, the where clause. A function rebuilt with `quote!` from single parts contains only the parts that the author remembered, and a forgotten one disappears without any error. On an error the macro returns its `compile_error!` and the unchanged item. With the item in place, the compiler can go on and check the code that uses the function, and reports mistakes there in the same run. Without it, those mistakes stay hidden until the argument is fixed, and tools that work on the expansion do not see the function at all.

</details>

## 06 · A function-like macro with its own syntax

<details><summary>Hint 1</summary>

Design two types first: one flag (attributes and a name) and the whole input (attributes, visibility, name, storage type, a `Punctuated` list of flags). Implement `Parse` for both, in the order in which the tokens come. Then write a separate function that checks the parsed value and collects every problem, and a third that generates. `flags` only connects the three.

</details>
<details><summary>Hint 2</summary>

`input.call(Attribute::parse_outer)?` reads any number of attributes, `input.parse::<Visibility>()?` reads a visibility (also an empty one), `input.parse::<Token![struct]>()?` the keyword. `let content; syn::braced!(content in input);` opens the braces, and `content.parse_terminated(Flag::parse, Token![,])?` reads the list. Parse the storage as a `syn::Type` and check afterwards whether it is a path that `is_ident("u8")` and so on; this gives your own error for `i32` and for `Vec<u8>`. For the generated constants, `proc_macro2::Literal::usize_unsuffixed(index)` prints `3`, not `3usize`. Write `::core::primitive::bool` for the return type of `contains`, and emit the storage type as `::core::primitive::#storage`, because the user's module may have its own `bool` or `u16`. The derives have paths too: `::core::clone::Clone`, `::core::marker::Copy`, `::core::cmp::PartialEq`, `::core::cmp::Eq`, `::core::fmt::Debug`. `all()` can be `Name(0 #(| Self::#flags.0)*)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

An iterator over tokens gives one token at a time and nothing else. A `ParseStream` is a cursor with knowledge of Rust syntax: `parse::<T>()` reads a whole `T` at the cursor, moves past it, and returns an error with a position when the tokens do not fit. `peek` looks at the next token without moving the cursor, so the parser can decide which branch to take. `parse_terminated` repeats one parser, separated by a punctuation token, until the stream is empty, and allows the separator after the last element. At the end the stream must be empty, or the whole parse fails with `unexpected token`. syn wrote every error about the *shape* of the input for me: a missing `:`, `enum` instead of `struct`, a missing comma. I wrote the errors about its *meaning*, which syn cannot know: a storage type that is not supported, a flag name used twice, more flags than the type has bits.

</details>

## 07 · Hygiene, crate paths, and how to test a macro

<details><summary>Hint 1</summary>

For task A, parse two `Expr` values with a comma between them, and write the expansion as one block with `quote_spanned! {Span::mixed_site()=> { .. } }`: a `let __attempt_max: ::core::primitive::u32 = #max;`, a counter, and a `loop` with a `match` on the expression. For task B, look at the item's `attrs` for `named`, read `crate = PATH` with `parse_nested_meta` into an `Option<syn::Path>`, and interpolate the path in front of `::Named`. For task C, parse the output of `expand` into a `syn::ItemImpl` and compare its parts with what the contract says, for three or four inputs.

</details>
<details><summary>Hint 2</summary>

Task A: `match #body { ::core::result::Result::Ok(__attempt_value) => break ::core::result::Result::Ok(__attempt_value), ::core::result::Result::Err(__attempt_error) => { if __attempt_tries >= __attempt_max { break ::core::result::Result::Err(__attempt_error); } } }`, with the counter raised before the `match`. Every name that the expansion declares, also in the two patterns, starts with `__attempt_`. Task B: `meta.path.is_ident("crate")` works although `crate` is a keyword; `meta.value()?.parse::<syn::Path>()?` reads the path; the default is `syn::parse_quote!(::m5_proc_macros)`. To report an unknown argument and still check the rest, record `meta.error(..)` in a variable outside the closure, read the argument's value if `meta.input.peek(Token![=])`, and return `Ok(())`. Task C: your inputs need at least a plain struct, a type with a lifetime, a bounded type parameter with a default and a where clause, and one input each with `#[named(crate = ::kit)]`. Check the trait path (`item.trait_`), `item.generics.params`, `item.self_ty`, the where clause, and the value of the constant `NAME`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

Tokens that the macro creates with `quote!` have the span `call_site`: every name in them is resolved as if the user had written it at the macro call. So all names in generated code are resolved in the user's module, except where the macro takes care. For local variables the protection is the span `mixed_site`: a variable declared with it can be named only by tokens with the same span, so the user's expression cannot see it and it cannot shadow a variable of the user. For paths to items the protection is an absolute path that starts with `::`. A name in a pattern is in between: if an item of that name is in scope, the pattern means the item, so generated variables also get names that no user item has, starting with `__`. A unit test on tokens compares identifiers by their letters. Two expansions that differ only in spans look the same to it. Whether a name resolves correctly is decided by the compiler, so only a test that compiles a real use of the macro, in a module full of colliding names, can show hygiene.

</details>

## 08 · Capstone: `#[derive(Builder)]`

<details><summary>Hint 1</summary>

Write the three generated items by hand for the example struct first, in a scratch file, and make them compile. Then design one plain struct per field that holds what the generator needs: the name, the field's type, the type of the setter's value, whether the field is an Option field, whether the setter converts, and what `build` does when the field was never set (missing, `None`, `Default::default()`, or an expression). Fill a list of these, collecting errors, and generate only from the list.

</details>
<details><summary>Hint 2</summary>

An Option field: `Type::Path` without `qself`, whose last segment is named `Option` and has `PathArguments::AngleBracketed` with exactly one `GenericArgument::Type`. The builder stores `::core::option::Option<VALUE>` for every field. The struct declaration takes `#generics` and `#where_clause` as they are; the two impls take the pieces of `split_for_impl`. In `build`, first collect the names of required fields that are `None` into a vector; if it is not empty, return the error. One way to get the values out without `unwrap` is a `let (Some(a), Some(b)) = (self.a, self.b) else { return Err(..) };` over the required fields, generated only when there is at least one. For a field with a default, `match self.field { Some(v) => v, None => FALLBACK }`. Generate the whole `build` function, its signature included, with `quote_spanned! {Span::mixed_site()=> .. }` so that EXPR cannot see its variables, and start every generated parameter and variable name with `__`. It has to be the whole function because `self` is hygienic like a local variable: a `self` parameter made by `quote!` and a `self.field` made with `mixed_site` do not meet, and the compiler reports E0424. A local name that you build yourself and interpolate keeps its own span, so make it with `Ident::new("__required_0", Span::mixed_site())`. Give `Default::default()` the span of the field's type, as in lesson 03.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The macro decides from syntax whether a field is optional: its type must be written as a path that ends in `Option<X>`. A type alias such as `type Maybe<T> = Option<T>;` is not recognised, so that field is required and its setter takes a `Maybe<T>`. A user's own type named `Option` is recognised by mistake. The macro cannot do better, because types are resolved after macros have run; the honest answer is to document the rule. It also decides from syntax which attribute arguments exist, and reports the others. User-written code runs in one place only: the expression of `default = EXPR` is pasted into the body of `build`. There the macro's own variables and the user's expression would share a scope. I generate that function with the span `mixed_site`, so its variables are invisible to EXPR, and I give all generated variables and parameters names that start with `__`, so that a constant or unit struct of the user cannot change what a pattern means. Library items are named by absolute paths throughout.

</details>

## C01 · Code review

<details><summary>Hint 1</summary>

Try the derive, on paper, on five inputs: a generic struct, a tuple struct, an enum, a struct with `#[reset(kep)]`, and a struct in a module that has its own trait called `Default`. Each one shows a different defect. Then look at how the body of `reset` is built, and ask what has happened to the spans of the user's field names by the time the body is tokens again.

</details>
<details><summary>Hint 2</summary>

Give `reset_impl` a helper that returns `syn::Result<TokenStream>`, and turn every `unwrap` and `panic!` into an error with a span: `new_spanned(data.enum_token, ..)` for an enum. Use `split_for_impl` for the header. For the field access, `syn::Member::Named(ident.clone())` or `syn::Member::Unnamed(syn::Index::from(i))` interpolates as `name` or as a bare number. Build each assignment with `quote_spanned! {field.ty.span()=> self.#member = ::core::default::Default::default(); }`. Read the attribute with `parse_nested_meta`, accept `keep`, and report everything else with `meta.error(..)`. Iterating with `.iter().enumerate()` removes clippy's warning about the index, and taking `&Field` instead of `&Vec<Attribute>` removes the other one.

</details>
<details><summary>Check your review</summary>

The derive ignores generic parameters, so every generic struct gets `impl Reset for Name` and fails with E0107. For a tuple struct it interpolates a `usize`, which prints as `0usize`; `self.0usize` is not valid, so every tuple struct fails. It treats any `#[reset(..)]` attribute as `keep`, so a typing mistake such as `#[reset(kep)]` is accepted and the field silently keeps its value, which is the worst of the bugs because nothing reports it. It panics on enums and unions, and on input that does not parse, so the user sees "proc-macro derive panicked" at the derive line. It builds the body as a string and parses it again; that loses every span, so an error about a field type without `Default` points at the derive, and the generated code is further from the user's source than it needs to be. `Default::default()` and `m5_proc_macros::Reset` are relative paths and break in a module that has its own `Default`, or where the crate is not in scope under that name. Smaller points: `input.ident.clone()` and `input.data.clone()` copy what could be borrowed, the loop indexes where it could iterate, and `is_kept` takes `&Vec<Attribute>` where a slice would do; clippy reports the last two.

</details>

## Drills

Run `cargo drill m5` in the repository root after lesson 03. The compiler output tells you *what* happened; these notes say *why*. Read a note only after you have seen the verdict for that drill. All nine are about the standard derives: before you write derives, know what the built-in ones generate.

<details><summary>d01 · The bound a derive adds (lesson 03)</summary>

`E0599`. `#[derive(Clone)]` on `Shared<T>` writes `impl<T: Clone> Clone for Shared<T>`: one bound per type parameter, without looking at the fields. `Session` is not `Clone`, so `Shared<Session>` has no `clone` method, although cloning an `Rc<T>` never clones a `T`. Write the impl by hand without the bound: `impl<T> Clone for Shared<T>` with `Shared(Rc::clone(&self.0))`. This is the "too strict" case of the per-parameter strategy.

</details>
<details><summary>d02 · Default for an enum (lesson 03)</summary>

`E0665`. A struct's derived default is made from the defaults of its fields. For an enum the derive would have to choose a variant, and it does not guess. Mark one unit variant with the helper attribute `#[default]`; here it must be `Safe`, because of the KEEP lines.

</details>
<details><summary>d03 · A helper attribute without its derive (lesson 03)</summary>

`error` (no code): `cannot find attribute default in this scope`. `#[default]` is a helper attribute that the derive `Default` declares. It is known only inside an item that has this derive. Add `Default` to the derive list.

</details>
<details><summary>d04 · Copy without Clone (lesson 02)</summary>

`E0277`. `Copy` is declared as `trait Copy: Clone`, and `#[derive(Copy)]` writes only `impl Copy for Id {}`. A derive adds exactly its own impl; it does not add impls for supertraits. Derive `Clone` as well.

</details>
<details><summary>d05 · The order a derive compares in (lesson 02)</summary>

`panic`. The derived `PartialOrd` compares the fields in the order in which they are declared and stops at the first that differs. With `patch` first, 1.9.4 is "greater" than 2.0.0, and the assertion fails at run time. Declare the fields as `major`, `minor`, `patch`. A derive sees a list of fields and cannot know which one is the most important.

</details>
<details><summary>d06 · A derive on a function (lesson 01)</summary>

`E0774`. `derive` may only stand on a struct, an enum or a union, because a derive adds an impl for a type. Remove the line. A macro that changes a function is an attribute macro (lesson 05).

</details>
<details><summary>d07 · A derive and a hand-written impl (lesson 01)</summary>

`E0119`. The derive adds an ordinary `impl Clone for Counter` next to the struct. Together with the hand-written impl there are two, and a type can implement a trait only once. Remove `Clone` from the derive list; the KEEP lines need the hand-written behaviour.

</details>
<details><summary>d08 · A derive behind cfg_attr (lesson 01)</summary>

`E0277`. `cfg_attr(test, derive(Debug))` becomes `derive(Debug)` only when the crate is compiled for tests. In a normal build the attribute disappears before any macro runs, and `Job` has no `Debug`. Derive it without the condition.

</details>
<details><summary>d09 · A bound on the impl, not on the type (lesson 03)</summary>

`ok`. The derive writes `impl<T: Debug> Debug for Wrapper<T>`. The bound is on the impl. `Wrapper<Opaque>` is a perfectly good type; it only has no `Debug`, and this program never asks for it. Compare with d01, where the program does ask.

</details>

## Mastery check: model answers

Answer the questions in the [module guide](README.md#mastery-check) from memory first. Then compare.

<details><summary>Model answers (after writing your own)</summary>

**1. The life of a derive macro.** A derive is a function in a crate with `proc-macro = true`. Such a crate can export only macros, so the trait that the derive implements is in another crate, which usually re-exports the derive. The compiler builds the macro crate first, for the machine that compiles. When it later meets `#[derive(Name)]` on an item in a user's crate, after the tokenizer has run and before types are checked, it calls the function with the item's tokens (attributes included; the `#[derive(...)]` attribute that names the macro is removed, and a derive attribute written below it is still there). The function returns tokens, and the compiler adds them after the item and goes on compiling the result as part of the user's crate. The macro sees syntax only. It cannot know what a type name refers to, whether a type implements a trait, what an alias stands for, or how large anything is.

**2. The impl header, and the two kinds of bound.**

```text
impl<'a, T: Clone> Trait for View<'a, T> where T: Send { .. }
```

After `impl`: the parameters with their bounds and without the default. After the name: only the parameter names. Then the where clause. One bound per type parameter (`T: Trait`) is wrong for `struct Shared<T>(Rc<T>)` when the trait is `Clone`: it demands `T: Clone` although no `T` is ever cloned. It is also wrong for `struct First<T: Iterator> { item: T::Item }`: the code uses a `T::Item`, and a bound on `T` says nothing about it. One bound per field type (`FieldType: Trait`) is wrong for a type that contains itself, such as `struct Node<T> { value: T, next: Option<Box<Node<T>>> }`: the bound `Option<Box<Node<T>>>: Trait` needs the impl that it is written on, and the compiler does not accept that circle, so `Node<u8>` ends up without the trait.

**3. A misspelled attribute argument.** The macro returns, as its normal output, the tokens of a `compile_error!("...")` call. Every token of that call carries the span of the misspelled argument, which the macro took from the token it was parsing (`meta.error(..)`, or `Error::new(ident.span(), ..)`). The compiler expands the call and prints the message under the source text of that span. A panic would also stop the build, but the compiler would print "proc-macro derive panicked" under the derive line, so the user would not learn where the mistake is and would suspect a bug in the macro. A unit test parses source text with `proc_macro2` (with the `span-locations` feature) and compares the error's message, line and column. A trybuild compile-fail case compares the compiler's whole output with a recorded `.stderr` file, which contains the same position as line, column and underline.

**4. Attribute macros and the end of a function.** The output of an attribute macro replaces the item. If the macro returns only an error, the function is gone: the compiler cannot check the code that uses it, so mistakes there stay hidden until the attribute is fixed, and tools that work on the expansion do not see the function. So the macro returns the error *and* the unchanged item. To run code at the end of a function, the macro can wrap the body in a closure and run its code after the call. That covers `return` and `?`, but not a panic, and it changes the body: variables are captured, and an `async` body needs more care. Or the macro can add one `let` at the top that creates a guard whose `Drop` does the work. Drop runs at the normal end, at `return`, at `?` and during a panic, and the body is untouched. Its limits: the guard must be bound to a real name (`let _ =` drops at once), and it cannot see the function's return value.

**5. Names in generated code.** Local variables: give them the span `Span::mixed_site()`. Then they belong to the macro alone, as in `macro_rules!`. Paths to items of the standard library: write them in full, starting with `::core` or `::std`, because every path is resolved in the user's module. The path to the macro's own crate: `::crate_name`, with `extern crate self as crate_name;` in the crate itself, and an attribute argument such as `crate = path` for users who reach the crate under another name. A name in a pattern (`let x`, a parameter, a `match` arm) means an item when a constant, unit struct or enum variant of that name is in scope. That is decided at the call site, where hygiene has no say, so generated names also start with `__` to be names that no item has. A unit test on tokens cannot check any of this for variables: two identifiers with the same letters and different spans compare as equal and print the same. Only the compiler resolves names, so only a compiled test case, in a module with colliding names, shows whether the macro is hygienic.

**6. The capstone again, with a changed requirement.** No model answer for the redo. For the example requirement, a missing required field as a compile error: the builder needs one type parameter per required field that records in the type whether the field is set (the typestate pattern of [`mastery/m1_types`](../m1_types/README.md)). `builder()` returns the builder with every marker "unset", each setter of a required field returns a builder with that marker changed to "set", and `build` exists only in the impl where all markers are "set". `build` then returns the value itself and not a `Result`, and `BuildError` is no longer needed. The price: the builder's type changes with every setter call, so a half-filled builder cannot be stored in a variable of one fixed type or filled in a loop, and the compiler's message for a missing field is "method `build` not found", which says less than `BuildError` did. The tests change with it: the run-time test for missing fields becomes a compile-fail case with a `.stderr` file, and that file is now part of the user experience that you design.

</details>
