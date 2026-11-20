# External macro consumer

[Chapter guide](../README.md) · [Concepts](../CONCEPTS.md)

This small workspace is the second check of lesson 06. After you implement the lesson and answer its `p_predictions`, run from the repository root:

```sh
cargo test --manifest-path exercises/chapter22/export_lab/Cargo.toml
```

The workspace has two crates and no registry dependencies.

- `library` contains nothing of its own: it includes your `l06_exports.rs` by path as the public module `l06_exports`. Its only dev-dependency is the repository's own `practice` crate, which the lesson's prediction test uses.
- `consumer` depends on the library under another name (`renamed_events`) and defines its own function called `format_event`. Its one test calls `renamed_events::ch22_event!`.

The consumer test passes only if your macro names its helper as `$crate::l06_exports::format_event`. A bare `format_event` finds the consumer's function. A path that starts with `crate::` means the consumer crate, which has no such module, so it does not compile. Inside the chapter's own test target these three spellings cannot be told apart at run time; that is why this workspace exists.

The library declares `pub mod l06_exports`. A `pub` function inside a private module cannot be reached from another crate. To see the error, remove that `pub` for a moment, run the command again, and restore it.

Until lesson 06 is implemented, the tests here fail at its `todo!()` and `guess()` lines. That is expected.
