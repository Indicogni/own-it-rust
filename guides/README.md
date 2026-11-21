# Guides

[Course overview](../README.md)

Short orientation pages that sit beside the course. They do not replace any chapter; they tell you where your earlier experience helps, where it misleads you, and which units to give the most time.

## New to the course

- [Start here](START_HERE.md): what the course is, who it is for, the three ways to set up (a dev container, GitHub Codespaces, or your own installation), your first hour, and where to get help.

## Coming from another language

Read the page for the language you know best before Chapter 3. Each one lists what transfers, what does not, the five mistakes people from that language make most often (with the real compiler message), the chapters and modules to prioritise, and a table that maps familiar idioms to Rust.

- [Coming to Rust from C and C++](FROM_C_AND_CPP.md)
- [Coming to Rust from Go](FROM_GO.md)
- [Coming to Rust from Python](FROM_PYTHON.md)
- [Coming to Rust from Java and C#](FROM_JAVA_AND_CSHARP.md)
- [Coming to Rust from JavaScript and TypeScript](FROM_JAVASCRIPT_AND_TYPESCRIPT.md)

## Working faster

- [Taming compile times](COMPILE_TIMES.md): `cargo check`, incremental builds, debug information, workspaces, generics, procedural macros, sccache, the linker, and how to measure.

Every Rust code block in these pages (the start page has none) is compiled and run with `rustdoc --edition 2024 --test` on the pinned toolchain, and every compiler message is copied from a real run.
