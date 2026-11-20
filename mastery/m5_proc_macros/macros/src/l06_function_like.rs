//! LESSON 06 — A function-like macro with its own syntax
//! Run: cargo test l06_
//! Read first: "Lesson 06" in CONCEPTS.md (the package directory).
//!
//! PREDICT before running: first answer `p_predictions` at the bottom (replace
//! each `guess()` with your answer). Then consider:
//! 1. The input of `flags! { ... }` is not valid Rust. At which stage must it
//!    still be acceptable to the compiler, and at which stage does it not?
//! 2. A parser has consumed `pub struct Perm`. How does it decide what must
//!    come next, and what does it say when something else comes?
//!
//! IMPLEMENT the contract below. Only the entry point `flags` is given. The
//! types that hold the parsed input, their `Parse` impls, the checks and the
//! code generation are yours to design. Keep the supplied tests and add one
//! edge case.
//! EXPLAIN afterward: How does a `ParseStream` differ from an iterator over
//! tokens? What do `parse`, `peek` and `parse_terminated` do with the cursor?
//! Which errors did syn write for you, and which did you have to write?
//!
//! COMPILER / TRANSFER LAB (optional): add a second accepted form,
//! `flags! { pub struct Perm: u8 { READ = 4, WRITE } }`, where a flag may name
//! its bit. Decide what a duplicate bit means and where its error points.
//!
//! # Contract
//!
//! Input, where `[...]` marks an optional part:
//!
//! ```text
//! flags! {
//!     [attributes] [visibility] struct NAME: STORAGE {
//!         [attributes] FLAG, [attributes] FLAG, ... [,]
//!     }
//! }
//! ```
//!
//! NAME and each FLAG are identifiers. STORAGE is one of `u8`, `u16`, `u32`,
//! `u64`, and always means the primitive integer type of that name. The list
//! of flags may be empty.
//!
//! Output, for `/// File rights.` `pub struct Perm: u8 { READ, WRITE, EXEC }`:
//!
//! - the tuple struct `pub struct Perm(u8);` with the input's attributes and
//!   visibility. Its one field is private. The type is `Clone`, `Copy`,
//!   `PartialEq`, `Eq` and `Debug`;
//! - one inherent `impl Perm` block (and nothing else) with
//!   - one `pub const` per flag, with the flag's attributes: the first flag is
//!     bit 0 (`Perm::READ.bits() == 1`), the second is bit 1, and so on;
//!   - `pub const fn empty() -> Perm`: no flag set;
//!   - `pub const fn all() -> Perm`: every declared flag set;
//!   - `pub const fn bits(self) -> u8`: the stored number;
//!   - `pub const fn contains(self, other: Perm) -> bool`: every flag of
//!     `other` is set in `self`;
//!   - `pub const fn with(self, other: Perm) -> Perm`: the flags of both;
//!   - `pub fn names(self) -> Vec<&'static str>`: the names of the set flags,
//!     in declaration order.
//!
//! Every function except `names` must be usable in a `const`.
//!
//! The output means the same in every module, also in one that has its own
//! items named `Vec`, `Clone`, `Debug`, `bool` or `u16`
//! (`tests/ui/l06/pass_hostile.rs`).
//!
//! Errors. Report all of them in one run, in source order, as `compile_error!`
//! calls and nothing else (no struct, no impl):
//! - STORAGE is another type: ``unsupported flag storage: use `u8`, `u16`, `u32` or `u64` ``
//!   spanned on the type;
//! - a flag name used before: ``duplicate flag `FLAG` `` spanned on the later one;
//! - more flags than STORAGE has bits: ``` `u8` has room for 8 flags ``` (with
//!   the real type and number) spanned on the first flag that does not fit,
//!   once. Flags are counted as written, a duplicate included. This check is
//!   skipped when STORAGE is unsupported;
//! - input that does not have the shape above: pass on syn's error.
//!
//! The macro must never panic.

use proc_macro2::TokenStream;

/// The function-like macro `flags!`. `input` holds the tokens between the
/// macro's delimiters. See the contract at the top of this file.
pub(crate) fn flags(input: TokenStream) -> TokenStream {
    todo!("Implement this task")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::{assert_absolute, at, compact, errors_in, guess, is_clean, parse, tokens};
    use syn::parse::Parser;
    use syn::punctuated::Punctuated;
    use syn::{Fields, Ident, ImplItem, Item, Token, Type, Visibility};

    const PERM: &str = "
/// File rights.
#[repr(transparent)]
pub struct Perm: u8 {
    /// May read.
    READ,
    WRITE,
    EXEC,
}";

    fn items(output: TokenStream) -> Vec<Item> {
        assert!(
            is_clean(&output),
            "valid input must expand without errors: {output}"
        );
        parse::<syn::File>(output, "a list of items").items
    }

    #[test]
    fn a_the_output_is_a_tuple_struct_and_one_impl() {
        let items = items(flags(tokens(PERM)));
        assert_eq!(
            items.len(),
            2,
            "the output is one struct and one impl block"
        );
        let Item::Struct(item) = &items[0] else {
            panic!("the first item of the output must be the struct");
        };
        assert_eq!(item.ident, "Perm");
        assert!(
            matches!(item.vis, Visibility::Public(_)),
            "the visibility is copied"
        );
        let Fields::Unnamed(fields) = &item.fields else {
            panic!("`Perm` must be a tuple struct");
        };
        assert_eq!(fields.unnamed.len(), 1);
        assert!(
            compact(&fields.unnamed[0].ty).ends_with("u8"),
            "the struct stores a `u8`"
        );
        assert!(
            matches!(fields.unnamed[0].vis, Visibility::Inherited),
            "the stored number is private: users go through `bits()`"
        );
        let attributes: Vec<String> = item.attrs.iter().map(compact).collect();
        assert!(
            attributes.iter().any(|attr| attr.contains("Filerights."))
                && attributes.iter().any(|attr| attr == "#[repr(transparent)]"),
            "the attributes of the input belong on the struct; found {attributes:?}"
        );
        let Item::Impl(block) = &items[1] else {
            panic!("the second item of the output must be the impl block");
        };
        assert!(block.trait_.is_none(), "an inherent impl");
        assert_eq!(compact(&block.self_ty), "Perm");
    }

    #[test]
    fn a_each_flag_is_a_public_constant_with_its_attributes() {
        let items = items(flags(tokens(PERM)));
        let Item::Impl(block) = &items[1] else {
            panic!("the second item of the output must be the impl block");
        };
        let constants: Vec<&syn::ImplItemConst> = block
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Const(constant) => Some(constant),
                _ => None,
            })
            .collect();
        let names: Vec<String> = constants.iter().map(|c| c.ident.to_string()).collect();
        assert_eq!(names, ["READ", "WRITE", "EXEC"]);
        for constant in &constants {
            assert!(
                matches!(constant.vis, Visibility::Public(_)),
                "flags are `pub const`"
            );
            let ty = compact(&constant.ty);
            assert!(
                ty == "Perm" || ty == "Self",
                "`{}` must be a constant of the flags type, written as `Perm` or `Self`; \
                 found `{ty}`",
                constant.ident
            );
        }
        assert!(
            constants[0]
                .attrs
                .iter()
                .any(|attr| compact(attr).contains("Mayread.")),
            "the doc comment of `READ` must be on its constant"
        );
        assert!(constants[1].attrs.is_empty(), "`WRITE` has no attributes");
    }

    #[test]
    fn a_functions_are_const_except_names() {
        let items = items(flags(tokens(PERM)));
        let Item::Impl(block) = &items[1] else {
            panic!("the second item of the output must be the impl block");
        };
        for name in ["empty", "all", "bits", "contains", "with", "names"] {
            let function = block
                .items
                .iter()
                .find_map(|item| match item {
                    ImplItem::Fn(function) if function.sig.ident == name => Some(function),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("the impl block has no function `{name}`"));
            assert!(
                matches!(function.vis, Visibility::Public(_)),
                "`{name}` is `pub`"
            );
            assert_eq!(
                function.sig.constness.is_some(),
                name != "names",
                "`{name}`: every function except `names` is a `const fn`"
            );
        }
    }

    #[test]
    fn a_small_variations_are_accepted() {
        for source in [
            "struct Empty: u64 {}",
            "pub(crate) struct One: u16 { A }",
            "struct NoComma: u32 { A, B }",
        ] {
            let output = flags(tokens(source));
            assert!(
                is_clean(&output),
                "`{source}` is valid; output was: {output}"
            );
        }
        let output = flags(tokens(PERM));
        assert_absolute(
            &output,
            &[
                "Vec",
                "Clone",
                "Copy",
                "PartialEq",
                "Eq",
                "Debug",
                "bool",
                "u8",
                "core",
                "std",
            ],
        );
    }

    #[test]
    fn b_unsupported_storage_points_at_the_type() {
        let source = "\nstruct Perm: i32 { READ }";
        assert_eq!(
            errors_in(flags(tokens(source))),
            [(
                "unsupported flag storage: use `u8`, `u16`, `u32` or `u64`".to_owned(),
                at(source, "i32")
            )]
        );
        let source = "\nstruct Perm: Vec<u8> { READ }";
        assert_eq!(errors_in(flags(tokens(source))).len(), 1);
    }

    #[test]
    fn b_duplicates_point_at_the_later_flag() {
        let source = "\nstruct Perm: u8 {\n    READ,\n    WRITE,\n    READ,\n}";
        let output = flags(tokens(source));
        assert_eq!(
            errors_in(output.clone()),
            [("duplicate flag `READ`".to_owned(), (5, 4))]
        );
        assert!(
            !compact(&output).contains("struct"),
            "on an error the output is the `compile_error!` calls only; output was: {output}"
        );
    }

    #[test]
    fn b_too_many_flags_point_at_the_first_that_does_not_fit() {
        let source = "\nstruct Wide: u8 { A, B, C, D, E, F, G, H, I, J }";
        assert_eq!(
            errors_in(flags(tokens(source))),
            [("`u8` has room for 8 flags".to_owned(), at(source, "I"))],
            "report the limit once, at the ninth flag"
        );
        let fits = "struct Full: u8 { A, B, C, D, E, F, G, H }";
        assert!(is_clean(&flags(tokens(fits))), "eight flags fit into a u8");
    }

    #[test]
    fn b_all_problems_are_reported_in_source_order() {
        let source = "\nstruct Perm: bool {\n    READ,\n    READ,\n    EXEC,\n    EXEC,\n}";
        assert_eq!(
            errors_in(flags(tokens(source))),
            [
                (
                    "unsupported flag storage: use `u8`, `u16`, `u32` or `u64`".to_owned(),
                    at(source, "bool")
                ),
                ("duplicate flag `READ`".to_owned(), (4, 4)),
                ("duplicate flag `EXEC`".to_owned(), (6, 4)),
            ],
            "one run must show every problem"
        );
    }

    #[test]
    fn b_syntax_errors_come_from_the_parser_and_never_panic() {
        for source in [
            "",
            "struct Perm { READ }",
            "struct Perm: u8",
            "enum Perm: u8 { READ }",
            "struct Perm: u8 { READ WRITE }",
            "struct Perm: u8 { READ } extra",
            "struct Perm: u8 { 7 }",
        ] {
            let output = std::panic::catch_unwind(|| flags(tokens(source))).unwrap_or_else(|_| {
                panic!("flags panicked on `{source}`: return a compile_error! instead")
            });
            assert_eq!(
                errors_in(output.clone()).len(),
                1,
                "`{source}` does not have the shape of the contract: pass on the parser's \
                 error; output was: {output}"
            );
        }
    }

    /// P. Predict BEFORE running: replace each `guess()` with your answer.
    #[test]
    fn p_predictions() {
        // P1. Is the keyword `struct` an identifier for `Ident`'s parser?
        let p1: bool = guess();
        assert_eq!(
            p1,
            syn::parse_str::<Ident>("struct").is_ok(),
            "P1: `struct` parses as an Ident (left: your prediction, right: actual)"
        );

        // P2. How many names are in this list, and does the list remember that
        // it ended with a comma?
        let list = Punctuated::<Ident, Token![,]>::parse_terminated
            .parse_str("A, B, C,")
            .expect("a valid list");
        let p2: (usize, bool) = guess();
        assert_eq!(
            p2,
            (list.len(), list.trailing_punct()),
            "P2: (length, trailing comma) (left: your prediction, right: actual)"
        );

        // P3. The parser for a type reads `u8`. One more token is left over.
        let p3: bool = guess();
        assert_eq!(
            p3,
            syn::parse_str::<Type>("u8 extra").is_ok(),
            "P3: `u8 extra` parses as a Type (left: your prediction, right: actual)"
        );

        // P4. And a visibility, parsed from no tokens at all?
        let p4: bool = guess();
        assert_eq!(
            p4,
            syn::parse_str::<Visibility>("").is_ok(),
            "P4: the empty input parses as a Visibility (left: your prediction, right: actual)"
        );
    }
}
