# Candidate review comments for pull request 33

Thirteen comments that reviewers drafted. Some describe real problems that must be
fixed before the merge. Some are true but small. Some are simply mistaken, and posting
them would cost the author time and cost you trust.

Judge each one in `src/l05_review.rs`, task B:

- **Blocking**: the pull request must not be merged until this is fixed. It breaks a
  merge requirement of `fixture/CONTRIBUTING.md`: wrong results, a panic that a user
  can cause, a breaking API change, a test that no longer protects anything, a
  missing changelog entry.
- **NonBlocking**: true and worth saying, but the pull request could be merged
  without it. Naming, wording, small improvements, questions.
- **Wrong**: the comment is mistaken. The code does not do what the comment says, or
  the advice would make the project worse.

## Patch 1: `builtin: add a sort step`

**A.** `let path = &args[0];` panics when the step has no argument. The other steps
return `StepError::usage(self)` in that case. Please do the same.

**B.** `ws.read(path).unwrap()` panics when the file does not exist. A typo in a
Lanefile must not crash the tool. Return an error, as `upper` does.

**C.** nit: `lines.sort()` could be `lines.sort_unstable()`. Two equal lines cannot be
told apart, so a stable sort gains nothing here.

**D.** `lines` borrows from `ws`, and `ws.write(...)` needs `&mut ws`. This cannot
compile. Did you build it?

**E.** A new step is a change that users can see. It needs an entry under
`## [Unreleased]` in `CHANGELOG.md`.

## Patch 2: `tests: make the summary test reliable`

**F.** This does not fix the problem, it hides it. The documentation of `summary()`
promises plan order, so the old assertion was right and the code is wrong. Please
fix `summary()` and keep the exact comparison.

**G.** Good change. A test should never compare a whole string; `contains` is always
the more robust way to assert on text.

## Patch 3: `registry: take &str in lookup`

**H.** This changes the return type of a public function from
`Option<&Box<dyn Step>>` to `Option<&dyn Step>`. Code that names the old type stops
compiling, so this cannot go into a 0.4.x release. Issue 27 asks for a path with a
deprecation: add a new method and keep `lookup` as it is.

**I.** Callers that pass a `&String` will stop compiling, because the parameter is
now `&str`.

## Patch 4: `glob: speed up matching`

**J.** `?` now matches one byte, not one character. `v?.txt` matched `vé.txt` before
this patch and does not match it now. Please keep the behaviour and add a test with a
non-ASCII path.

**K.** nit: "occurence" should be "occurrence" in the new comment.

**L.** `pattern[p]` on a byte slice is an unchecked access. It needs an `unsafe` block
with a `// SAFETY:` comment.

**M.** "Much faster" in the commit message: do you have a number? A before and after
measurement would make the claim easy to trust. Not blocking if the change is
otherwise right.
