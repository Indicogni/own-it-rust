# 21: A way to see what `run` would do

**Labels:** `feature`, `accepted` · **State:** open

---

**ops-lena** opened this issue

We run tasklane in CI with Lanefiles that are generated from templates. Before a
real run I would like to know that the plan is fine and that no variable is missing,
without creating any file. `tasklane plan` gives me the order, but it does not check
the steps and the variables. `tasklane check` does not take a task.

Proposal: a dry-run mode. I started on a patch that adds a `dry_run: bool` parameter
to `Runner::run` and a new status `TaskStatus::Planned`, but I wanted to ask first.

---

**ada** (maintainer) commented

Yes to the feature, no to that shape. Both changes would break every user of the
library: `run` is called with four arguments everywhere, and `TaskStatus` is matched
without a wildcard arm in a lot of code, including ours. We cannot do that in 0.4.x.

What we would accept:

- **Library:** a setting on `RunOptions`, next to `keep_going`:
  `RunOptions::dry_run(self, on: bool) -> Self` and
  `RunOptions::is_dry_run(&self) -> bool`. `RunOptions` was made
  `#[non_exhaustive]` with private fields for exactly this.
- **Behaviour:** a dry run does everything a real run does *before* the first step:
  it plans, looks up every step, and expands every argument, so it reports the same
  errors. Then it runs nothing. The files of the workspace are untouched. The report
  lists every planned task, in plan order, as `TaskStatus::Skipped` with the message
  `dry run` and `steps_run == 0`. No new status.
- **Command line:** `-n` / `--dry-run` for `run`, documented in `--help` and in the
  README table.
- Tests in our suite, documentation on the new methods, and a changelog entry under
  `Added`.

Does that cover your use case?

---

**ops-lena** commented

It does. I will not get to it soon, though. If somebody else wants to take it,
please go ahead.
