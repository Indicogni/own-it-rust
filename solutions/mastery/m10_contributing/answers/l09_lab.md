# Lesson 09 lab notes: building Clippy from source

Fill in what you saw on your machine. The test checks the form of each line, not the
values: your commit and your numbers will differ from anybody else's.

Commit: ddecc8599b65b25351726777db63c727903c9ba8

Toolchain: nightly-2026-11-17

UI test result: test result: ok. 4 passed; 1237 filtered out

One thing I learned about the layout: A lint lives in `clippy_lints/src/`, and its UI test is a pair of files with the same name in `tests/ui/`: the `.rs` input and the `.stderr` file with the expected diagnostics.
