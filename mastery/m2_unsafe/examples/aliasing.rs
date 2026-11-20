//! LESSON 06, task M — Specimens for the aliasing rules.
//!
//! Each function below compiles and returns a number. Some of them break the
//! aliasing rules, so **run them only under Miri**: a native run of a program
//! with undefined behaviour proves nothing. Do not repair them.
//!
//! For each specimen: read it, predict Miri's verdict under both models, then
//! run both commands (from the repository root; replace `a` with the name):
//!
//!   cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example aliasing -- a
//!   MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly-2026-11-20 miri run --manifest-path mastery/m2_unsafe/Cargo.toml --example aliasing -- a
//!
//! The first command uses Stacked Borrows, Miri's default model. The second
//! uses Tree Borrows. "Accepted" means that the program printed its result.
//! "Rejected" means that Miri stopped with `error: Undefined Behavior`.
//! Record each pair in the `m_miri_verdicts` test of `src/l06_aliasing.rs`.
//!
//! `demo1` and `demo2` are explained in CONCEPTS.md; use them to practise
//! reading Miri's output before you start on `a` to `h`.

// The `unsafe` blocks below have no SAFETY comments on purpose: for most of
// them no true comment could be written. That is what you are here to judge.
#![allow(clippy::undocumented_unsafe_blocks)]

use std::cell::Cell;

/// Worked example 1: a reference is used again after a pointer was made from it.
fn demo1() -> i32 {
    let mut x = 1;
    let r = &mut x;
    let p = r as *mut i32;
    unsafe { *p += 1 };
    *r += 1;
    unsafe { *p += 1 };
    x
}

/// Worked example 2: the owner is written while a raw pointer is in use.
fn demo2() -> i32 {
    let mut v = [1, 2, 3];
    let p = v.as_mut_ptr();
    v[0] = 10;
    unsafe { *p += 1 };
    v[0]
}

/// a. A pointer made from a shared reference is used to write.
fn a() -> i32 {
    let x = 1;
    let r = &x;
    let p = std::ptr::from_ref(r).cast_mut();
    unsafe { p.write(2) };
    x
}

/// b. Two copies of one raw pointer take turns.
fn b() -> i32 {
    let mut x = 1;
    let p = &mut x as *mut i32;
    let q = p;
    unsafe {
        *p += 1;
        *q += 1;
        *p += 1;
    }
    x
}

/// c. A pointer to the first element is moved to the second element.
fn c() -> i32 {
    let mut v = [1, 2, 3];
    let first = &mut v[0] as *mut i32;
    unsafe { *first.add(1) = 20 };
    v[1]
}

/// d. A pointer made from a shared reference to a `Cell` is used to write.
fn d() -> i32 {
    let cell = Cell::new(1);
    let r = &cell;
    let p = r.as_ptr();
    unsafe { *p = 7 };
    r.get()
}

/// e. A raw pointer writes, the owner is read, the raw pointer writes again.
fn e() -> i32 {
    let mut x = 1;
    let p = &mut x as *mut i32;
    unsafe { *p = 2 };
    let seen = x;
    unsafe { *p = 3 };
    seen + x
}

fn bump(exclusive: &mut i32, alias: *mut i32) -> i32 {
    unsafe { *alias += 1 };
    *exclusive
}

/// f. A function gets a `&mut` and a raw pointer to the same value.
fn f() -> i32 {
    let mut x = 1;
    let p = &raw mut x;
    bump(unsafe { &mut *p }, p)
}

/// g. A reference to the slot just past an array is created and never read.
fn g() -> i32 {
    let v = [1, 2, 3];
    let p = v.as_ptr();
    let _end: &i32 = unsafe { &*p.add(3) };
    v[0]
}

/// h. Two pointers from two calls of `as_mut_ptr` on the same array.
fn h() -> i32 {
    let mut v = [1, 2, 3];
    let first = v.as_mut_ptr();
    let second = unsafe { v.as_mut_ptr().add(1) };
    unsafe {
        *first = 10;
        *second = 20;
    }
    v[0] + v[1]
}

fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    let result = match name.as_str() {
        "demo1" => demo1(),
        "demo2" => demo2(),
        "a" => a(),
        "b" => b(),
        "c" => c(),
        "d" => d(),
        "e" => e(),
        "f" => f(),
        "g" => g(),
        "h" => h(),
        _ => {
            eprintln!("usage: aliasing <demo1|demo2|a|b|c|d|e|f|g|h>");
            std::process::exit(2);
        }
    };
    println!("specimen {name} returned {result}");
}
