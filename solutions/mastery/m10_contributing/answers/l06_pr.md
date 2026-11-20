<!-- Lesson 06, task B: the description of the pull request for your fix of issue 14.
     The template is from fixture/CONTRIBUTING.md, section "Pull requests". -->

# vars: keep the character after a lone `$`

## What

`expand` no longer drops the character that follows a `$` when the `$` starts neither
`${NAME}` nor `$$`. The change is a few lines in `crates/tasklane-core/src/vars.rs`, a
regression test next to the other `expand` tests, and a changelog entry.

## Why

Fixes #14. A step argument such as `coffee $3, tea $2` became `coffee $, tea $`. The
documentation of `expand` says that such a `$` "is kept as it is", so the code was
wrong and the documentation was right. The smallest input that shows it is `$3`.

Nothing else changes: `$$` is still an escape, an unknown variable and a missing `}`
are still errors, and values are still inserted without a second expansion.

## Testing

- `cargo test --workspace`: all tests pass, including the new
  `lone_dollar_keeps_the_next_character`, which fails without the fix with
  `left: "$"`, `right: "$3"`.
- By hand: the Lanefile from the issue, with `tasklane run menu --show files`, and
  the same file with `$$` and `${SHOP}` in the text, to see that both still work.
- `cargo clippy --workspace --all-targets` prints no warning.

## Checklist

- [x] Tests added or updated
- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace --all-targets` is clean
- [x] `CHANGELOG.md` updated, or not needed (say why)
