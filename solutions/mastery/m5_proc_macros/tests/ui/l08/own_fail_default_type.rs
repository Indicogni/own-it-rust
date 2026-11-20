//! Lesson 08, own compile-fail case (model answer): `default = EXPR` with an
//! expression of the wrong type, and `default` on a type without `Default`.
//! The macro cannot check types; the compiler does, in the generated code.
//! This case records where those errors point.
//! This program must NOT compile. The expected compiler output is in
//! `own_fail_default_type.stderr`.

use m5_proc_macros::Builder;

struct NoDefault;

#[derive(Builder)]
struct Job {
    #[builder(default = "thirty")]
    timeout: u64,
    #[builder(default)]
    handle: NoDefault,
}

fn main() {}
