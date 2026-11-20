//! Lesson 08: `Builder` on inputs that are not a struct with named fields.
//! This program must NOT compile. The expected compiler output is in
//! `fail_shapes.stderr`.

use m5_proc_macros::Builder;

#[derive(Builder)]
struct Pair(u8, u8);

#[derive(Builder)]
struct Unit;

#[derive(Builder)]
enum Level {
    Low,
    High,
}

fn main() {}
