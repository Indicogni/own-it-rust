//! The library half of the key-value store. It is empty on purpose.
//!
//! This is a blank-file project: SPEC.md says what the program must do, and
//! you decide which modules, types and functions do it. The acceptance tests
//! only start the `kvstore` binary and talk to it over TCP, so nothing in
//! this crate has a name or a signature that you must keep.
//!
//! A common layout is to put the real code here, in modules, and to keep
//! `src/main.rs` down to "read the arguments, call the library, turn the
//! result into an exit code". Unit tests then live next to the code they
//! check. If you would rather build everything in the binary, delete this
//! file and the `[lib]` table in `Cargo.toml`.
