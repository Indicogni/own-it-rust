# 25: `concat docs/*.md` skips some files

**Labels:** `bug` · **State:** open

---

**tomasz-w** opened this issue

**Version:** 0.4.0

**Lanefile:**

```text
task docs
  step write docs/intro.md A
  step write docs/notes.md.md B
  step write docs/v1.2.md C
  step concat all.md "docs/*.md"
```

**Command:** `tasklane run docs --show files`

**Expected:** `all.md` contains the three files: `A`, `B`, `C` on three lines
(5 bytes).

**Actual:** `file all.md (1 bytes)`. Only `intro.md` is in it. `notes.md.md` (yes,
the double extension is a mistake in our export, but the file exists) and `v1.2.md`
are skipped without any message.

It looks as if a file is skipped when the text after the `*` (here `.md`) appears
more than once in the name, or when a part of it does: `v1.2.md` has a second `.`.
`remove "docs/*.md"` leaves the same files behind.

---

**mira** (maintainer) commented

Reproduced. The documentation of `glob::matches` says that `*` matches any number of
characters, so both files must match. Whoever fixes this: `*` and `?` must still
never match a `/`, and the pattern must still cover the whole path (`*.md` must not
match `a.md.txt`).
