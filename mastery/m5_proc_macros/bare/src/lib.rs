//! Mastery module M5, lesson 01: derive macros written with `proc_macro` only.
//!
//! This file is the part of a procedural macro crate that the compiler sees:
//! one public function per macro, each marked with an attribute that gives the
//! macro its name. You do not need to edit this file. The work is in
//! `l01_raw_tokens.rs`.
//!
//! Open `Cargo.toml` in this directory: `[dependencies]` is empty. The crate
//! `proc_macro` is not a dependency. It comes with the compiler, and only a
//! crate with `proc-macro = true` can use it.

// Starter bodies are `todo!()`, so parameters and imports stay unused until you
// implement them.
#![allow(dead_code, unused_variables, unused_imports)]

mod l01_raw_tokens;

use proc_macro::TokenStream;

/// `#[derive(TypeName)]` implements `m5_proc_macros::TypeName`.
#[proc_macro_derive(TypeName)]
pub fn derive_type_name(item: TokenStream) -> TokenStream {
    l01_raw_tokens::derive_type_name(item)
}

/// `#[derive(FieldNames)]` implements `m5_proc_macros::FieldNames`.
#[proc_macro_derive(FieldNames)]
pub fn derive_field_names(item: TokenStream) -> TokenStream {
    l01_raw_tokens::derive_field_names(item)
}
