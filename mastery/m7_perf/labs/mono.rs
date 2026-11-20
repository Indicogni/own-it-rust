//! Lab 6: how many copies of a function the program contains. See LABS.md.
//!
//! Build: rustc --edition 2024 -O --crate-type=lib --emit=llvm-ir -C codegen-units=1 -o mono.ll mono.rs

use std::fmt::Display;

/// Generic: the compiler makes one copy for every type it is called with.
#[inline(never)]
pub fn framed_generic<T: Display>(value: T) -> String {
    let text = value.to_string();
    let line = "-".repeat(text.len() + 4);
    format!("{line}\n| {text} |\n{line}")
}

/// A trait object: one copy serves every type.
#[inline(never)]
pub fn framed_dyn(value: &dyn Display) -> String {
    let text = value.to_string();
    let line = "-".repeat(text.len() + 4);
    format!("{line}\n| {text} |\n{line}")
}

/// A thin generic shell around a core that is not generic.
pub fn framed_shell<T: Display>(value: T) -> String {
    #[inline(never)]
    fn core(text: String) -> String {
        let line = "-".repeat(text.len() + 4);
        format!("{line}\n| {text} |\n{line}")
    }
    core(value.to_string())
}

pub fn callers() -> usize {
    let generic = [
        framed_generic(1u8),
        framed_generic(2u32),
        framed_generic(3i64),
        framed_generic(4.5f64),
        framed_generic('c'),
        framed_generic("text"),
    ];
    let dynamic = [
        framed_dyn(&1u8),
        framed_dyn(&2u32),
        framed_dyn(&3i64),
        framed_dyn(&4.5f64),
        framed_dyn(&'c'),
        framed_dyn(&"text"),
    ];
    let shell = [
        framed_shell(1u8),
        framed_shell(2u32),
        framed_shell(3i64),
        framed_shell(4.5f64),
        framed_shell('c'),
        framed_shell("text"),
    ];
    generic.len() + dynamic.len() + shell.len()
}
