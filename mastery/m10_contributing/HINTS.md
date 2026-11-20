# M10 hints

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Notes](NOTES.md)

Try the first hint before you open the second. Open *Check your explanation* only
after you have written your own answer to the lesson's EXPLAIN prompt. Complete
solutions are in `solutions/mastery/m10_contributing/` in the repository root.

## Lesson 01 · Orienting in an unfamiliar codebase

<details><summary>Hint 1</summary>

Start with `fixture/crates/tasklane-core/src/lib.rs`: its doc comment has a table of
the modules in pipeline order. For task A, the "home" is the module where the
decision is made, not where its result is printed. For exit codes, ask which crate
can call `std::process` or return a code at all. For task B, list the test names
(`cargo test --workspace -- --list` inside `fixture/`) and search the list for a word
from the rule: `outside`, `builtin`, `optional`, `windows`.

</details>
<details><summary>Hint 2</summary>

For task C, read three functions in order: `run_with` and `run_task` in
`tasklane-cli/src/lib.rs`, then `Runner::run` in `tasklane-core/src/run.rs`. Note
where `cli.sets` is applied (before the runner is even created), that `Runner::run`
calls `plan` first and then prepares *all* tasks before any step runs, and that the
command line prints the log only after `run` has returned. For P5: an `optional`
failure blocks nothing and does not stop the run; an ordinary failure, without
`--keep-going`, stops everything after it.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`main` in tasklane-cli hands the arguments to `tasklane_cli::run`, which parses them
(`args`), reads the Lanefile and calls `tasklane_core::parse`, with `text` splitting
words and quotes. For `run`, the command line builds the variables from the file
and the `--set` options and creates a `Runner`. `Runner::run` asks `graph::plan` for
the order, then looks up every step in the `registry` and expands every argument
with `vars::expand` before the first step runs, so a mistake anywhere is found
before the workspace changes. Then the steps (in `builtin`, behind the `Step` trait
of tasklane-plugin) run task by task, and `report` collects the results; the command
line prints log, files and summary and maps the outcome to an exit code. Text search
was fastest for messages, the test list for rules, and "find references" for
following the calls.

</details>

## Lesson 02 · Reproducing a bug report

<details><summary>Hint 1</summary>

The reporter says "the prices are gone" and quotes `coffee $, tea $`. So the
Lanefile's tasks, patterns and variables are noise: delete all tasks but `drinks`,
then all steps but one, then words from that step. Then go down a layer: the step
argument reaches `tasklane_core::expand`. Call it directly with shorter and shorter
templates until removing any character hides the problem.

</details>
<details><summary>Hint 2</summary>

The minimal template is a `$` followed by one ordinary character. In `expand`, look
at the `match chars.next()`: `next` *takes* the character after the `$` out of the
iterator before the code knows whether it belongs to the `$`. In the last arm the
character is thrown away. Decide on `${` and `$$` by looking ahead (a clone of the
iterator, or `peekable`), and consume only in those two arms. Do not touch the
arms for `{` and for the second `$`: the `b_unchanged_` tests describe them.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The bug in one sentence: `expand("$3", ..)` must return `$3`, because the
documentation says a `$` that starts neither `${` nor `$$` is kept as it is, but
0.4.0 returns `$`: it drops the character after such a `$`. Each `b_unchanged_` test
guards a rule that a careless fix would break. Treating every `$` as literal breaks
the `$$` escape and `$${A}`. "Being generous" with errors turns `${NOPE}` or an
unterminated `${` into text, which hides typos in Lanefiles. Re-scanning the output
would expand a `${...}` that came from a value. And a fix that steps over one *byte*
after the `$` breaks on a multi-byte character such as `é`. The regression test in
the fixture matters because the acceptance tests of this lesson are not part of the
project.

</details>

## Lesson 03 · Regression hunting with git bisect

<details><summary>Hint 1</summary>

The probe is an integration test, so it sees only the public API:

```rust
#[test]
fn hash_inside_an_argument_is_kept() {
    let file = tasklane::parse("task t\n  step write a.txt color=#ff0000\n").unwrap();
    assert_eq!(file.tasks[0].steps[0].args, ["a.txt", "color=#ff0000"]);
}
```

Save it as `tests/probe.rs` in the practice repository and do not `git add` it. Check
it by hand first: `git checkout v0.1.0` and `cargo test --test probe` must pass;
`git checkout main` and the same command must fail.

</details>
<details><summary>Hint 2</summary>

`cargo test` exits with 101 for a failed test and for a failed build alike. The
script needs two steps: `cargo build --quiet 2>/dev/null || exit 125`, then
`cargo test --quiet --test probe`, whose exit code becomes the script's. Run it with
`git bisect start v0.2.0 v0.1.0` and `git bisect run sh /full/path/l03_bisect.sh`,
then `git bisect reset`. For task D, the loop at the end of the "Three ways it goes
wrong" part of CONCEPTS prints one line per commit that does not build.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The first test of the plain run landed on a commit in the middle of a refactoring
whose library does not compile. `cargo test` failed there with exit code 101, the
same code as for a failed test, so git marked the commit bad and searched the older
half, where every commit that builds is good. It ended at the first commit that does
not build and blamed it, although that commit only moves code. Exit code 125 tells
`git bisect run` that a commit can be neither called good nor bad; git then tests a
neighbour. This works only while most commits build: if the commits around the real
culprit are skipped, git can only report a range. That is one reason why projects
want every commit, not only every pull request, to build and pass its tests.

</details>

## Lesson 04 · Making a change the maintainers will accept

<details><summary>Hint 1</summary>

For the predictions, read `downstream/pinned/src/lib.rs`: each comment there says
which freedom of the 0.4.0 API it uses. Then look at the attribute above each type
in the fixture: `#[non_exhaustive]` or not, public fields or private. For the
feature, read `RunOptions::keep_going` and copy its shape: one private field, one
setter that takes and returns `self`, one getter, all documented (the crate denies
`missing_docs`). The command line needs four small edits in
`tasklane-cli/src/args.rs` (field, default, the `match` on options, the help text)
and one in `lib.rs`. A new public field on `Cli` is allowed for the same reason as
a new setting on `RunOptions`: read the attribute above the struct.

</details>
<details><summary>Hint 2</summary>

In `Runner::run`, everything before `let mut report` is the checking a dry run must
still do: `plan`, and `prepare` for every task. Add the dry-run branch right after
it: for every prepared task push a `TaskResult` with `TaskStatus::Skipped`,
`message: Some("dry run".to_string())` and `steps_run: 0`, with `fatal` false, and
return the report. Do not run `run_task`. Then: a test in
`crates/tasklane-core/tests/run.rs`, the option in the README table, and under
`## [Unreleased]` in the changelog a `### Added` group with an entry that ends in
`(#21)`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

`RunOptions` is `#[non_exhaustive]` and its fields are private. Code outside the
crate can neither build it with a struct literal nor read a field, only call its
methods, so a new field and a new method cannot break anyone. `TaskStatus` is a
plain public enum, and users may `match` on it without a wildcard arm; a new variant
makes every such `match` a compile error (E0004). That is why the dry run reports
`Skipped` with a message. If the feature had truly needed a new status, the honest
options would have been to say so in the issue and wait for 0.5, or to find a
compatible detour, for example a separate method on the report. Marking `TaskStatus`
as `#[non_exhaustive]` now is itself a breaking change, for the same reason.

</details>

## Lesson 05 · Code review, both sides

<details><summary>Hint 1</summary>

For task A, `ci.txt` names the failing test; open that test in the fixture and ask
which patch changes what it counts. For "panic", look for `unwrap`, `expect` and
`[...]` indexing on things that come from a Lanefile. For "breaks downstream",
compare each changed signature with `downstream/pinned/src/lib.rs`. For "silently",
ask of the rewritten function: for which inputs is a byte not a character?

</details>
<details><summary>Hint 2</summary>

For task B, test the claims. D: copy the `Sort` step into the fixture or a scratch
file and build it. I: write a function that takes `&str` and call it with a
`&String`. L: is indexing a slice with `[]` ever undefined behaviour, or does it
panic? Then hold each true comment against the seven merge requirements in
`fixture/CONTRIBUTING.md`: if it names one, it blocks. For task C, read who wrote
each comment and what exactly they ask for; the test for comment 1 shows the exact
log lines.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A model review: patch 1 has two panics that a Lanefile can trigger (A, B), no
changelog entry (E), and turns CI red because a test counts the steps; C is a nit and
D is wrong, the code compiles. Patch 2 hides bug 30 by weakening the only assertion
that pinned the documented order (F blocks; G praises exactly that mistake). Patch 3
changes a public return type in a patch release (H blocks); I is wrong because
`&String` coerces to `&str` at a call. Patch 4 changes `?` from one character to one
byte (J blocks); K and M are fair and small; L is wrong, since slice indexing is
checked and panics. A bonus finding that no candidate comment has: `sort` drops the
last newline of the file (your prediction P2). Declining is helpful when the
alternative is a pull request that does two things, one of which cannot be merged:
saying no, with the reason and a pointer to issue 27, keeps the dry run mergeable and
keeps the other idea alive in the right place.

</details>

## Lesson 06 · Writing for maintainers

<details><summary>Hint 1</summary>

Open `fixture/CONTRIBUTING.md` next to your editor and write the commit message rule
by rule: area and colon, an imperative verb, at most 60 characters, no full stop;
blank line; body lines of at most 72 characters; `Fixes #14` as the last line. If the
test reports a problem, `commit_problems` in the lesson file shows exactly how each
rule is checked. For the predictions, apply the same rules to the three samples and
count; a subject can break several rules at once.

</details>
<details><summary>Hint 2</summary>

The body of the commit should contain the minimal case itself (`$3` became `$`), so
that somebody who meets the bug can find the commit with `git log --grep`. In the
pull request, `## Testing` wants the command (`cargo test --workspace`), the name of
your regression test and what it checks, and what you tried by hand. For the issue
report, reuse your reproduction from lesson 02 one layer up: a Lanefile of two or
three lines and one `tasklane` command. Expected and actual are quotations: the
sentence from the documentation of `expand`, and the output you got.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The commit message holds what stays true for as long as the code exists: what a user
saw, what the cause was, why this fix is right, and which issue it closes. The pull
request adds what only the review needs: how to test the change, what you tried by
hand, what you left out on purpose, the checklist. If the reason for a change is
only in the pull request, it is lost to everybody who works from the git history.
In the issue report, the two sentences that save the most time are the minimal
reproduction ("`step write menu.txt coffee $3` writes `coffee $`") and the expected
result with its source ("the documentation of `expand` says such a `$` is kept as it
is"). With those two, a maintainer can write the failing test without running
anything.

</details>

## Lesson 07 · Unsafe code and soundness reports

<details><summary>Hint 1</summary>

The poor report mentions the `slice` step, wrong numbers and a file that was shorter
than expected. Read `Slice` in `tasklane-core/src/builtin.rs` and follow the call
that takes the numbers. You are looking for a *safe* function with an `unsafe`
block. Then list the requirements of each unsafe call in that block from the
standard library's documentation (`slice::from_raw_parts`, "Safety"; and
`str::from_utf8_unchecked`) and compare the list with what the `if` above the block
checks.

</details>
<details><summary>Hint 2</summary>

The check covers `start` only. Choose a short content and a `len` that makes
`start + len` larger than the content's length, by a little: one byte is enough, and
the lesson refuses very large values. Run the LAB command and read Miri's message.
For the fix, `str::get(start..end)` returns `None` when the range is out of bounds
*or* cuts a character, and borrows from the original string, so the pointer test
still passes; compute `end` with `checked_add`. The advisory template names the
sections; the description must say that safe code is enough to trigger the problem.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The comment claimed: `start` is not after the end, and the bytes come from a
`String`, so they are UTF-8. The block really needs more: the whole range
`start..start + len` must lie inside the string's allocation, the addition must not
overflow, and the range must begin and end on character boundaries, because a slice
of valid UTF-8 is valid UTF-8 only if it is cut between characters. Miri sees the
first problem: building a slice that leaves the allocation is undefined behaviour in
the language itself, and Miri stops at `from_raw_parts`. Miri does not see the
second: a `&str` with half a character is a broken promise of the standard library,
not of the language, so nothing is reported until some later code relies on it. For
that case only a test of the returned value helps.

</details>

## Lesson 08 · Dependencies and upgrades

<details><summary>Hint 1</summary>

`cargo tree --manifest-path mastery/m10_contributing/fixture/Cargo.toml -d` answers
task A. For task B, the crate's repository has a `CHANGELOG.md`; find the heading
for version 2.0.0 and read "Major changes". For task C, change the version in
`fixture/Cargo.toml` under `[workspace.dependencies]`, and make
`tasklane-cli/Cargo.toml` say `bitflags.workspace = true`. Then build inside
`fixture/` and let the compiler lead you.

</details>
<details><summary>Hint 2</summary>

The changelog's list of traits to derive goes on `TaskFlags` inside the `bitflags!`
block, as an attribute above `pub struct`. Derive all of them, not only the ones the
fixture's own code needs: `--test downstream pinned` uses the rest. The field `.bits`
is now the method `.bits()`. Then run the fixture's tests: one of them fails with
`taskflags(...)` in the output. `describe_flags` builds text from `{:?}`; replace
that with `flags.iter_names()`, which yields the name of each set flag in declaration
order.

Now run `--test downstream pinned`. If it reports E0599 for `from_bits_unchecked`:
bitflags 1 generated that method and bitflags 2 does not, and no code of tasklane
called it, so the fixture built without it. The "Methods" part of the 2.0.0
changelog shows the few lines that bring it back; put them into `impl TaskFlags`,
with documentation and a `# Safety` section, because the crate denies
`missing_docs` and clippy asks for the section. The pinned crate then builds with a
deprecation warning, which is what its author should see. Finish with the changelog:
the upgrade under `### Changed` with `(#19)`, and the deprecated method under
`### Deprecated`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

The compiler found what is visible in types: the `bits` field that became a method,
and the traits that the macro no longer derives, as soon as the fixture's own code
copied, compared or printed a `TaskFlags`. Only a test found that the `Debug` output
changed from `SILENT | HIDDEN` to `TaskFlags(SILENT | HIDDEN)`, which changed what
`tasklane list` prints. And only a crate outside the repository found the traits that
tasklane itself never uses, such as `Hash` and `Ord`, and the method
`from_bits_unchecked`, which bitflags 2 dropped. `TaskFlags` is generated by the
dependency's macro and is part of tasklane's public API, so what the macro generates
*is* tasklane's API: a breaking release of bitflags is a breaking change of tasklane
unless tasklane restores everything by hand. That is why the upgrade needed more
care than its small diff suggests.

</details>

## Lesson 09 · Contributing to the wider ecosystem

<details><summary>Hint 1</summary>

Each doc comment in `process` names its source. The RFC README has a section "What
the process is"; search it for "calendar days". The two bot commands are on the
Forge under Triagebot: "Issue Assignment" and "Shortcuts". The ACP is described on
the first page of the std-dev-guide's "feature lifecycle". For Clippy, search its
CONTRIBUTING.md for "changelog". For the editions, run `rustc --help -v` with the
pinned toolchain and read the line for `--edition`; `future` is not an edition.

</details>
<details><summary>Hint 2</summary>

For the predictions, read the policy's "Rules" part: it has three lists, marked
allowed, banned, and allowed with caveats, and the last one says that every use in
it must be disclosed. Place each scenario in a list by its wording: "you are the
only one who sees the output"; "comments ... issue bodies and PR descriptions";
"documentation ... includes ... doc-comments"; "typo fixes"; "discover bugs, as long
as you personally verify". For the lab notes, `git rev-parse HEAD` prints the commit
and `rust-toolchain.toml` has the channel.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A new standard-library function starts as a problem statement and usually as an API
change proposal: an issue in `rust-lang/libs-team`, which the library API team
accepts if it expects the idea to succeed. Then comes a pull request to
`rust-lang/rust` with the implementation marked unstable behind a feature gate, and
a tracking issue that collects the open questions. People try the function on
nightly. When it has proved itself, somebody proposes stabilisation; the team runs a
final comment period, the stabilisation pull request is merged, and the function
reaches stable through nightly and beta with the next release train. From then on it
can never be removed, which explains the care. What surprises many people: how much
is decided in writing and in public, that a merged RFC is not a promise of
implementation, and how specific the rules about LLM use are.

</details>

## Lesson 10 · Capstone

<details><summary>Hint 1</summary>

Pick the issue whose kind of problem you have met least. Issue 22: generate the
ladder Lanefile in a test (a loop that pushes two lines per level) and plan the top
task; choose a depth that a correct planner finishes at once, and give the test a
time limit so that it fails instead of hanging (CONCEPTS, "Capstone", step 3). Then
read `visit` in `graph.rs` and ask when a task is walked a second time. Issue 25:
write the failing cases from the issue as `assert!(matches(..))` lines, then trace
`*.md` against `notes.md.md` on paper. Issue 30: run the test ten times, then look
at the type of the field that `summary` iterates over.

</details>
<details><summary>Hint 2</summary>

Issue 22: a task that is already in `order` was walked completely, with everything
below it; return early at the top of `visit`. Keep the cycle check working: the
`path` check must still run for tasks that are *not* finished. Issue 25: when the
rest of the pattern fails after a `*`, the star must be allowed to take one more
character and try again. Remember the position after the last star and how far it
reaches, and never let it step over a `/`. Issue 30: `results` is already in plan
order; build the list of failures from it and delete the hash map. Then the
changelog (`### Fixed`), `answers/l10_commit.txt` and `answers/l10_pr.md` with
`Fixes #N`.

</details>
<details><summary>Check your explanation (after writing your own)</summary>

A model story, for issue 22. The report: planning a generated Lanefile in which every
stage depends on both tasks of the stage before never finishes since 0.4.0, and the
time doubles with each stage. The reproduction: a unit test in `graph.rs` that
builds a ladder of 60 levels, plans the top task on a helper thread, and fails when
no plan has arrived after five seconds. The cause: `visit` walks a
task's dependencies every time the task is reached and only checks at the end
whether the task is already planned, so each level doubles the work of the level
below. The fix: return early when the task is already in the plan. Nothing else
changed, because a planned task was walked without finding a cycle, so a second
walk cannot find one; the order tests, the cycle tests, the whole suite and the
downstream crate pass as before.

</details>

## Mastery check: model answers

<details><summary>1. Orienting in an hour</summary>

Build and test first. Read README and CONTRIBUTING, then the manifests and
`cargo tree` for the crates and their direction of dependency. Read `lib.rs` of the
core crate, or the `cargo doc` front page, for the modules. Follow one input from
`main` to the output with "go to definition". For a behaviour, search the test names
for its key word and read that test.

</details>
<details><summary>2. From a report to a test</summary>

Reproduce it as reported. Halve the input until every part is needed. Move down to
the lowest function that still shows it. Take the expected result from the
documentation. Write the test, watch it fail for that reason. Before fixing, list
the documented behaviours around it and make sure each has a test that passes on the
old code.

</details>
<details><summary>3. Bisecting</summary>

`git bisect start BAD GOOD`, a probe test in an untracked file, and
`git bisect run` with a script: build first and `exit 125` if that fails, then run
the probe; 0 is good, 1 to 127 except 125 is bad. Read the commit that comes out,
and `git bisect reset`.

</details>
<details><summary>4. Does it break a user?</summary>

Ask what a user is allowed to write against the current API and whether it still
compiles: calls (parameters), struct literals and exhaustive matches (unless
`#[non_exhaustive]`), trait implementations (required methods), named types
(signatures). Compatible designs: a private field with a setter on an options
type, a defaulted trait method, a new function next to a deprecated old one.

</details>
<details><summary>5. Reviewing and being reviewed</summary>

Judge against the project's written rules; what breaks users, hides a failure or
breaks a rule blocks, the rest is marked as a suggestion. Run a claim before posting
it. As the author, answer every comment as done, answered or declined; decline what
is out of scope or breaking, with a reason and a place for the idea.

</details>
<details><summary>6. Writing</summary>

Commit: `area: imperative summary` within the project's limit, blank line, what the
user saw and why the fix is right, the issue reference. Pull request: what, why, how
tested, checklist. Issue: a title with input and effect, a minimal reproduction as
text, expected with its source, actual, version.

</details>
<details><summary>7. Unsound functions and Miri</summary>

A safe function is unsound if some input makes it cause undefined behaviour. Show it
with a test of safe code that reaches the bad case, run under Miri. Miri checks the
language's rules on the code that runs; it does not see broken library rules such as
invalid UTF-8 in a `&str`, and it does not see paths the tests do not take.

</details>
<details><summary>8. A breaking upgrade</summary>

Read the changelog of the major release. Change the requirement in one place. Fix
what the compiler reports. Run the tests for changes in behaviour, such as `Debug`
output. If the dependency's types are in your public API, restore everything users
could do before, and check with a crate from outside. Commit the lock file and write
a changelog entry.

</details>
<details><summary>9. How a change reaches stable Rust</summary>

Small fixes: a reviewed pull request. Substantial changes: an RFC (or an ACP for
library API), decided with a final comment period of ten calendar days; an
implementation on nightly behind a feature gate with a tracking issue; a
stabilisation report and a second FCP; then the release train. The rules are in the
rustc-dev-guide, the std-dev-guide, the Forge and the RFC repository. On LLMs:
private use is fine; LLM-written comments, descriptions, documentation and
diagnostics are banned; a few uses need disclosure; LLM-written code only inside the
described experiment.

</details>
<details><summary>10. The capstone again</summary>

There is no model answer for a second issue beyond the files in `solutions/`. The
measure is the rubric in the README: reproduce first, minimal test, one-sentence
cause, smallest fix, nothing else changed, and texts that a maintainer can act on.

</details>
