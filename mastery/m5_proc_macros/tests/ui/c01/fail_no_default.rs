//! Code review lesson: a field without `Default`.
//! This program must NOT compile. The expected compiler output is in
//! `fail_no_default.stderr`. The error is the compiler's, about the generated
//! code. It must point at the field, which only works when the generated
//! assignment is built from the field's own tokens.

use m5_proc_macros::Reset;

struct Handle;

#[derive(Reset)]
struct Job {
    id: u32,
    handle: Handle,
}

fn main() {}
