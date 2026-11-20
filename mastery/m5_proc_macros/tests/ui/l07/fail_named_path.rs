//! Lesson 07, task B: a `crate` path that leads nowhere.
//! This program must NOT compile. The expected compiler output is in
//! `fail_named_path.stderr`. The macro cannot know which paths exist: it
//! writes the path into the output, and the compiler checks it there. Because
//! the path's tokens are the user's own, the error points at what the user
//! wrote.

use m5_proc_macros::Named;

#[derive(Named)]
#[named(crate = crate::no_such_module)]
struct Lost;

fn main() {}
