//! Lesson 08: `default = EXPR` is the user's code. It must not reach into the
//! generated `build` function, and that includes the function's `self`.
//! This program must NOT compile. The expected compiler output is in
//! `fail_default_self.stderr`.
//!
//! If this program compiles, or if the compiler reports another error, EXPR
//! can see the `self` of `build`. Then `build` was written with the span of
//! the macro call. Generate the whole function, its `self` parameter
//! included, with `Span::mixed_site()`. This check does not depend on the
//! names that you chose for the local variables of `build`.

use m5_proc_macros::Builder;

#[derive(Builder)]
struct Server {
    #[builder(default = self.note.is_some())]
    noted: bool,
    note: Option<String>,
}

fn main() {}
