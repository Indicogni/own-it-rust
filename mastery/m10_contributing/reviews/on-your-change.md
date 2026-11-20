# Review of your pull request 34: "run: add a dry-run mode"

You opened a pull request with your change from lesson 04. Three people commented.
Lesson 05, task C: act on the comments in the fixture where that is right, and answer
every comment in `answers/l05_replies.md`.

---

**Comment 1** — Ada (maintainer), on `crates/tasklane-core/src/run.rs`

> Thanks, the shape is what we agreed on in #21. One thing before I can merge: a dry
> run that prints only `0 done, 0 failed, 5 skipped` does not tell me *what* would
> run. Please add one line per step to the workspace log, in this form:
>
> ```text
> [TASK] would run: STEP ARGS...
> ```
>
> with the arguments after variable expansion, in plan order, and nothing for tasks
> with the `silent` flag: the same rule as for a real run. Then `tasklane run site
> --dry-run` shows the whole plan.

**Comment 2** — Jun (maintainer), on `RunOptions::dry_run`

> Why a setter on `RunOptions` and not a `dry_run: bool` parameter on `Runner::run`?
> A parameter would be harder to overlook. I am not asking you to change it; I want
> to understand the reasoning and have it written down in the pull request.

**Comment 3** — nightowl42 (a user, not a maintainer), on `crates/tasklane-core/src/run.rs`

> While you are touching this file: `self.registry.lookup(&call.name)` is so ugly.
> Can you change `Registry::lookup` to take `&str` and return `&dyn Step` in this
> pull request too? It is a five-line change.

**Comment 4** — Ada (maintainer), general

> Please add a test that uses `--dry-run` (or `dry_run`) together with keep-going. I
> want to be sure that the two settings do not get in each other's way.
