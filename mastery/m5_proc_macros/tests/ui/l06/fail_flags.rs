//! Lesson 06: the errors of `flags!`.
//! This program must NOT compile. The expected compiler output is in
//! `fail_flags.stderr`.

use m5_proc_macros::flags;

// Three problems in one invocation, reported together.
flags! {
    struct Perm: i32 {
        READ,
        WRITE,
        READ,
        WRITE,
    }
}

flags! {
    struct Wide: u8 { A, B, C, D, E, F, G, H, I, J }
}

fn main() {}
