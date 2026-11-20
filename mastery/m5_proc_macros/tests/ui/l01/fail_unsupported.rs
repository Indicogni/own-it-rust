//! Lesson 01, task B: every error points at the item's name.
//! This program must NOT compile. The expected compiler output is in
//! `fail_unsupported.stderr`.

use m5_proc_macros::{FieldNames, TypeName};

#[derive(TypeName)]
struct Wrapper<T>(T);

#[derive(FieldNames)]
struct Generic<'a> {
    text: &'a str,
}

#[derive(FieldNames)]
struct Pair(u8, u8);

#[derive(FieldNames)]
enum Level {
    Low,
    High,
}

#[derive(FieldNames)]
struct Unit;

fn main() {}
