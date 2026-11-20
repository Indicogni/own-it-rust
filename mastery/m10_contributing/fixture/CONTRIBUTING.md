# Contributing to tasklane

Thank you for helping. This file says how the project works and what a pull request
needs before it can be merged. It is short on purpose; please read all of it.

## Building and testing

```text
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo doc --workspace --no-deps
```

All four must pass without warnings before you open a pull request. CI runs the same
commands.

One test is known to fail from time to time: see issue 30. If a test that you did
not touch fails, run it again and look at the issue list before you spend time on it.

## Where things are

`crates/tasklane-core/src/lib.rs` has a table of the modules, in the order in which a
Lanefile passes through them. Each module has unit tests at its end; tests that use
only the public API are in `crates/*/tests/`. The examples in `examples/` are run by
a test, so they cannot go stale.

## Merge requirements

A pull request is merged when a maintainer has approved it and all of these hold.
Reviewers will block on them:

1. **The tests pass**, including the ones you did not write.
2. **A change of behaviour has a test** that fails without the change. A bug fix
   comes with a regression test; a feature comes with tests for its rules and for
   its edge cases.
3. **No breaking change in a 0.4.x release.** Code that compiles against 0.4.0 must
   compile against 0.4.1. In practice: do not change or remove a public signature, do
   not add a parameter, do not add a variant to an enum or a field to a struct that
   is not `#[non_exhaustive]`, do not add a required method to a trait. If a change
   needs one of these, open an issue first; it will wait for 0.5.
4. **A change that users can see has a changelog entry** under `## [Unreleased]` in
   `CHANGELOG.md`, in the right group (`Added`, `Changed`, `Deprecated`, `Fixed`,
   `Security`), with the issue number in parentheses.
5. **Public items have documentation.** The crates deny `missing_docs`, so the
   compiler tells you. Options of the command also go into the table in `README.md`
   and into `--help`.
6. **Steps never panic on user input.** A wrong argument or a missing file is a
   `StepError`. No `unwrap`, `expect` or unchecked indexing on anything that comes
   from a Lanefile.
7. **New `unsafe` code needs a reason and a `// SAFETY:` comment** that states what
   must be true and why it is. Expect questions. If safe code is fast enough, use
   safe code.

Everything else in a review (names, style, small improvements) is a suggestion. A
reviewer marks such comments with `nit:` or says that they do not block.

## Commit messages

```text
vars: keep the value of an empty variable

`set EMPTY =` followed by `${EMPTY}` gave an "unknown variable" error,
because the parser dropped variables with an empty value. An empty
value is a value. Keep it, and add a test for the round trip.

Fixes #8
```

- **Subject:** `area: summary`. The area is the part of the project: `parse`,
  `vars`, `glob`, `plan`, `run`, `report`, `registry`, `builtin`, `plugin`, `cli`,
  `docs`, `tests` or `deps`. The summary starts with a verb in the imperative ("fix",
  "add", "keep", not "fixed", "adds", "keeping"). At most 60 characters in total, no
  full stop at the end.
- **A blank line.**
- **Body:** what the user saw, with the input that went wrong, and why this change is
  the right fix. Lines of at most 72 characters. Do not list the files you changed;
  the diff does that.
- **Last line:** `Fixes #N` for the issue that the commit closes.

One commit does one thing. A fix and an unrelated clean-up are two commits, and
usually two pull requests.

## Pull requests

Use this template for the description:

```markdown
# area: summary

## What

What the pull request changes, in two or three sentences.

## Why

The problem it solves. `Fixes #N`.

## Testing

The commands you ran, what the new tests check, what you tried by hand.

## Checklist

- [ ] Tests added or updated
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets` is clean
- [ ] `CHANGELOG.md` updated, or not needed (say why)
```

Keep a pull request small. If a reviewer asks for something that is outside its
purpose, it is fine to say so and to open an issue for it.

## Reporting a bug

A good report has: a title that names the input and what happens to it; the smallest
Lanefile and command that show the problem; what you expected, and why; what happened
instead; the output of `tasklane --version`. If you cannot make the Lanefile small,
send what you have and say so.

Please report a problem that could be a security problem privately to the
maintainers first, not in a public issue.

## Dependencies

tasklane has few dependencies and pins each one to an exact version with `=`, so
that every build of a release uses the same code. An upgrade is a pull request like
any other: read the dependency's changelog, adapt the code, and check that nothing
changes for users of tasklane.

## Conduct and licence

Be kind and assume good intent; comment on the code, not on the person. By sending a
pull request you agree that your contribution is licensed under the MIT licence, like
the rest of the project.
