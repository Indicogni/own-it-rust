# M10 concepts: working in real codebases and contributing

[Module guide](README.md) · [Hints](HINTS.md) · [Notes](NOTES.md)

Read one section before each lesson. The sections teach the way of working; the
lessons make you do it on `fixture/`, the tasklane project.

The commands in this file were run for this course, and every output shown is
real, shortened where marked with `...`. Paths were shortened to start at
`fixture/`.

## What you will be able to do

**After this module you can** find your way in a Rust project you did not write,
reproduce and fix a reported bug without breaking something else, find the commit
behind a regression, shape a change so that maintainers can accept it, review code
and answer a review, write commit messages, pull requests, issues and advisories,
recognise and report unsound code, upgrade a dependency across a breaking release,
and take a first step into the Rust project itself.

## Before you start

Assumed: chapters 1 to 24 of this course, and everyday git (commit, branch, diff,
log). Miri and the compatibility rules of public APIs are explained here as far as
the lessons need them; modules [M2](../m2_unsafe/) and [M8](../m8_api/) go deeper.

## Time

Estimates for a first pass, reading included: lessons 01 to 03 and 06
about 2 hours each; lessons 04, 05 and 07 about 3 hours each; lesson 08 2 to 3 hours;
lesson 09 3 to 4 hours, of which the lab is about one; the capstone about 4 hours.

## Lesson 01 · Orienting in an unfamiliar codebase

You cannot read a whole project before you change it, and you do not need to. You
need a **map with three layers**, and you build each layer with a different tool.

1. **Packages.** Which crates exist, and which depends on which? The manifests and
   `cargo tree` tell you. Dependencies point from the outside in: the command-line
   crate depends on the library, never the other way round. So the dependency
   direction tells you where the core is.
2. **Modules.** What is each module for? Read the top of `lib.rs` first. Good
   projects put a module table there, and `cargo doc --no-deps --open` shows it as
   the front page with every public item one click away.
3. **One path.** Follow a single input from the entry point (`main`, or the public
   function that the README shows) to the output. Once you have walked one path, the
   other code has a place to hang.

Before any of this, **build it and run it**. Run the test suite, run the example from
the README, break the input on purpose and read the error. Ten minutes of using a
program tell you which words its authors use, and those words are your search terms.

A reading order that works for most Rust projects: README, CONTRIBUTING, the
manifests, `lib.rs`, one example, then the tests of the part you care about.

### Worked example: where does this message come from?

A user shows you the error `task b depends on unknown task missing` and asks whether
it is raised while parsing or while planning. Search for a piece of the text that
cannot be a variable:

```text
$ grep -rn "depends on unknown task" crates/
crates/tasklane-core/src/parse.rs:83:                    format!("task `{}` depends on unknown task `{dep}`", task.name),
crates/tasklane-core/src/parse.rs:277:            (3, "task `b` depends on unknown task `missing`".to_string())
```

Two hits answer more than the question. Line 83 is the code: the parser raises it.
Line 277 is a test, and a test is the most reliable documentation a project has,
because it cannot go stale without somebody noticing. Read the test to learn the
exact rule (here: the error is reported at the line of the task that names the
missing dependency).

You can also start from the tests. The list of test names is a list of the rules the
authors cared about:

```text
$ cargo test --workspace -- --list 2>/dev/null | grep -i optional
an_optional_failure_blocks_nothing: test
```

And the package layer, in one command:

```text
$ cargo tree -p tasklane-cli
tasklane-cli v0.4.0 (fixture/crates/tasklane-cli)
├── bitflags v2.13.2
├── tasklane-core v0.4.0 (fixture/crates/tasklane-core)
│   └── tasklane-plugin v0.4.0 (fixture/crates/tasklane-plugin)
│       └── bitflags v1.3.2
└── tasklane-plugin v0.4.0 (fixture/crates/tasklane-plugin) (*)
```

Read it from the bottom: `tasklane-plugin` depends on nothing of the project, so it
is the foundation. (You will come back to the two bitflags lines in lesson 08.)

### Editor navigation

With rust-analyzer, four commands replace most searching: *go to definition*, *find
all references* (who calls this?), *go to implementations* on a trait (which types
implement `Step`?), and *workspace symbol search* (jump to any item by name). Use
text search for strings and messages, and the editor for names.

One trap: a name you import is not always defined where you import it from.
Libraries re-export items of other crates. The compiler knows the real home, and
`std::any::type_name` will tell you (it is meant for diagnostics; do not build logic
on its exact text):

```rust
fn defining_crate<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>().split("::").next().unwrap_or("")
}

// You write `std::string::String`, but the type lives in the `alloc` crate
// and `std` re-exports it.
assert_eq!(defining_crate::<String>(), "alloc");
assert_eq!(defining_crate::<std::path::PathBuf>(), "std");
```

### Mistakes people really make

- **Reading top to bottom.** Files are not chapters. Follow calls, not line numbers.
- **Trusting a comment over a test.** Comments describe what somebody intended at
  some time. A passing test describes what the code does today.
- **Changing code they cannot predict.** Before you edit a function, predict what it
  returns for one input and check. If you are wrong, you have not understood it yet,
  and you found that out cheaply. The `p_predictions` tests train this habit.
- **Not running it.** Many questions ("what does it print when the file is missing?")
  take five seconds to answer by trying and twenty minutes by reading.

### When to use which

| You have | Use |
| --- | --- |
| a message or other literal text | text search (`grep -rn`, or your editor's search) |
| a name (function, type, trait) | go to definition, find all references |
| a rule ("X is an error") | the list of test names, then the test |
| no idea where to start | `cargo doc --open`, then the README example in a debugger or with `dbg!` |

**Read more:** [`cargo tree`](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) ·
[`cargo doc`](https://doc.rust-lang.org/cargo/commands/cargo-doc.html) ·
[how rustdoc documents re-exports](https://doc.rust-lang.org/rustdoc/write-documentation/re-exports.html) ·
[`std::any::type_name`](https://doc.rust-lang.org/std/any/fn.type_name.html)

## Lesson 02 · Reproducing a bug report

A bug report is a claim: "with this input I expected A and got B". Often one of the
three parts is missing, and the input is much larger than it needs to be. Your work
before any fix is to turn the report into a **minimal failing test**: the smallest
input that still shows the problem, the expected result taken from the
documentation, and an assertion.

Why minimal? A small case points at the cause. A 30-line input fails for one reason
and passes through twenty functions on the way; a 2-character input passes through
two. Minimal cases also make the best regression tests, because the next reader can
see what they check.

**How to reduce.** Work like a binary search, by hand:

1. Reproduce the report exactly as given. If you cannot, stop and ask; do not guess.
2. Delete about half of the input. Does the problem still show? Keep the deletion.
   Does it vanish? Undo and delete the other half.
3. Repeat until every remaining part is necessary: removing any character makes the
   problem go away.
4. Move down a layer. If the problem shows in the command-line tool, call the
   library function that the tool calls; then the function below that.

**What "expected" means.** Not what you feel is right: what the project promises. Look
for the promise in the documentation of the function, in the README, in the
changelog, or in an existing test. If there is no promise, the report may be a
feature request, and that is a different conversation.

### Worked example

A report says: "The initials on our name badges are wrong for some people. List of
400 names attached." The function:

```rust
/// The first letter of every word of `name`, in upper case.
fn initials(name: &str) -> String {
    name.split(' ')
        .map(|word| word.chars().next().unwrap_or('?').to_ascii_uppercase())
        .collect()
}

assert_eq!(initials("ada okafor"), "AO");
```

Halving the list leads to one name, `Jun  Tanaka`, with two spaces. Shrinking the
name leads to `a  b`: remove either letter or one space and the result is right. The
documentation says "every word", and an empty string between two spaces is not a
word. That gives the test:

```rust,should_panic
# fn initials(name: &str) -> String {
#     name.split(' ')
#         .map(|word| word.chars().next().unwrap_or('?').to_ascii_uppercase())
#         .collect()
# }
// The minimal reproduction. It fails:
assert_eq!(initials("a  b"), "AB");
```

```text
assertion `left == right` failed
  left: "A?B"
 right: "AB"
```

Now the cause is one step away: `split(' ')` yields an empty piece between two
spaces. The fix is `split_whitespace()`.

### A fix must change one thing

The second half of the lesson is about what the fix must **not** do. Suppose you
"fix" `initials` by dropping every character that is not a letter. `a  b` passes.
But `R2 D2` was `RD` before and still is, while `2pac shakur` changes from `2S` to
`S`, and nobody asked for that. A fix that is broader than the bug changes behaviour
that other users depend on.

So before you touch the code, write down what must stay the same, and check it
afterwards. In the lesson these are the `b_unchanged_` tests. In a real project you
find them by reading the function's documentation line by line and asking for each
sentence: is there a test for this? If not, add one first and see it pass on the old
code.

### Mistakes people really make

- **Fixing where the symptom shows.** The badge printer could strip `?` from the
  output. The bug would still be in `initials` for every other caller.
- **Taking "expected" from intuition.** Then the maintainer disagrees, and the work
  is lost. Quote the sentence of the documentation that the code breaks.
- **A fix without a test.** The same bug will come back with the next refactoring,
  and nobody will know that it was fixed once.
- **A test that passes before the fix.** Always run the new test against the old code
  and watch it fail, for the reason you expect. A test that never failed proves
  nothing.

**Read more:** [`str::split`](https://doc.rust-lang.org/std/primitive.str.html#method.split)
and [`str::split_whitespace`](https://doc.rust-lang.org/std/primitive.str.html#method.split_whitespace) ·
[how to write tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) ·
[`should_panic` in doc tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html#attributes)

## Lesson 03 · Regression hunting with git bisect

A **regression** is something that worked in an older version and does not work now.
That gives you two commits: a good one and a bad one. Somewhere between them is the
first bad commit, and `git bisect` finds it by binary search. With 1000 commits in
between it needs about 10 tests, not 1000.

You give git three things:

- a **bad** commit (usually the newest),
- a **good** commit (an old release tag),
- a way to decide good or bad at any commit.

The decision can be a command. `git bisect run COMMAND` checks out a commit, runs the
command, reads its **exit code**, and continues by itself:

| Exit code | Meaning for `git bisect run` |
| --- | --- |
| 0 | this commit is good |
| 125 | this commit cannot be tested; skip it |
| 1 to 127, except 125 | this commit is bad |
| anything else | stop the bisection |

### Worked example: when did a message change?

In the practice repository of this lesson, old versions say `dependency cycle
through` and the newest says `dependency cycle: a -> b -> a`. Which commit changed
it? The test is a text search; `!` turns "found" into exit code 1, which means bad:

```text
$ git bisect start v0.2.0 v0.1.0
Bisecting: 15 revisions left to test after this (roughly 4 steps)
[55acd2bc1fa4db9e2274f46b16cbdf3e409dcabb] refactor: move expand() into its own module (2/3)
$ git bisect run sh -c '! grep -rq "dependency cycle: " src'
...
commit aeafdbdd6c8bba17c13f47eb7492ef03a3049e3f
Author: Jun Tanaka <jun@tasklane.example>
Date:   Fri Mar 6 12:00:00 2026 +0000

    plan: show the whole cycle in the error
...
bisect found first 'bad' commit
$ git bisect reset
```

Always finish with `git bisect reset`. It returns you to the branch you started on.

When the thing you look for is a piece of text, there is a shortcut. The "pickaxe"
option lists the commits that added or removed a string:

```text
$ git log --oneline -S'dependency cycle: ' -- src
aeafdbd plan: show the whole cycle in the error
```

Bisect is for the other cases: when you know the behaviour that changed but not the
text that changed it.

### The probe test

For a behaviour you need a test, and the test for a new report does not exist in old
commits. Put it in a file that git does not track, for example `tests/probe.rs`.
Untracked files stay in the work tree while git checks out commit after commit. Keep
the probe small and use only API that exists in the whole range; if the oldest good
commit cannot compile your probe, the bisection has nowhere to start.

### Three ways it goes wrong

**Commits that do not build.** `cargo test` exits with code 101 when a test fails
*and* when the code does not compile. To `git bisect run` both are "bad". If the
search lands on a commit with a broken build, git walks towards the commit that
broke the build and blames it for your regression. The answer is a script that
builds first and exits with 125 when the build fails. You will see both outcomes in
the lesson. To list the commits of a range that do not build:

```sh
for c in $(git rev-list --reverse v0.1.0..v0.2.0); do
  git checkout -q "$c" && cargo build -q 2>/dev/null \
    || git log -1 --format='does not build: %h %s' "$c"
done
git checkout -q main
```

**A flaky test.** If the probe fails only sometimes, one false "good" sends the
search into the wrong half. Make the probe deterministic first, or run it several
times inside the script.

**A dirty work tree.** Uncommitted changes to tracked files block checkouts or travel
with you through history. Commit or stash before you start. The one exception is the
untracked probe.

When the search ends, **read the commit**: `git show`. The first bad commit is
evidence, not a verdict. A commit that only moves code cannot change behaviour; if
bisect blames one, your test measured something else.

**Read more:** [git bisect](https://git-scm.com/docs/git-bisect) (the section
"Bisect run" has the exit codes) ·
[cargo-bisect-rustc](https://rust-lang.github.io/cargo-bisect-rustc/), the same idea
for regressions of the compiler itself

## Lesson 04 · Making a change the maintainers will accept

A maintainer who merges your change takes on its cost for as long as the project
lives: reading it, keeping it working, answering questions about it. A change is
accepted when it is clearly cheaper to merge than to discuss. Four things make it
cheap:

- **It was agreed before it was written.** For anything larger than a bug fix, the
  design is settled in an issue first. If a maintainer wrote "we would accept this
  shape", build that shape, even if you would have chosen another.
- **It fits the existing design.** It uses the extension points that are already
  there and follows the conventions of the code around it.
- **It is complete.** Tests in the project's own suite, documentation on every public
  item, a changelog entry, and whatever else CONTRIBUTING lists.
- **It breaks nobody.** Code that compiled against the last release compiles against
  the next one.

### What "breaks" means

Rust projects follow semantic versioning as Cargo understands it. Cargo treats
`0.4.0` and `0.4.1` as compatible and will update users from one to the other
without asking. `0.4` to `0.5` (or `1.x` to `2.0`) is a breaking release, which users
choose. So within `0.4.x`, every program that compiled must still compile.

These changes look small and break users. Each error is what a user of the fixture
gets (lesson 04 lets you predict them, and each one was compiled for this course):

| Change to a public item | A user's code that stops compiling | Error |
| --- | --- | --- |
| new parameter on a function | every call | E0061 |
| new variant on an enum | a `match` without a wildcard arm | E0004 |
| new public field on a struct | a struct literal | E0063 |
| new required method on a trait | every `impl` of the trait | E0046 |

The same mechanics in a few lines. A user matched all variants, as they are allowed
to; then the library adds one:

```rust,compile_fail,E0004
pub enum Status {
    Done,
    Failed,
    Planned, // added in a "small" update
}

// The user's code, written when there were two variants:
fn word(status: Status) -> &'static str {
    match status {
        Status::Done => "done",
        Status::Failed => "failed",
    }
}
```

```text
error[E0004]: non-exhaustive patterns: `Status::Planned` not covered
```

### Designs that can grow

Library authors prepare for growth, and you can recognise the preparation in the
source:

- **`#[non_exhaustive]` on an enum** forces code *outside the crate* to have a
  wildcard arm, so a new variant breaks nobody. On a struct, it forbids struct
  literals outside the crate, so a new field breaks nobody.
- **Private fields with methods.** Nobody outside can name the fields, so they can
  change freely. A settings type built this way grows by one method per setting:

```rust
#[derive(Debug, Default, Clone)]
pub struct Options {
    retries: u32,
    verbose: bool, // added later; no caller had to change
}

impl Options {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn retries(mut self, count: u32) -> Self {
        self.retries = count;
        self
    }
    pub fn verbose(mut self, on: bool) -> Self {
        self.verbose = on;
        self
    }
}

// Code written before `verbose` existed still compiles:
let options = Options::new().retries(3);
assert!(!options.verbose);
```

- **Trait methods with a default body.** Existing implementations do not have to
  write the new method.

When your feature seems to need a breaking change, look for one of these doors
first. A new setting goes into the options type. A new outcome can often be an
existing status with a message. A better function can live next to the old one,
with `#[deprecated]` on the old one. If no door fits, say so in the issue and let
the maintainers decide; the change then waits for the next breaking release.

### Mistakes people really make

- **"It is only a `bool` parameter."** It is a compile error in every caller.
- **A drive-by clean-up in the same pull request.** It doubles the review and makes
  the history harder to bisect. One pull request, one purpose.
- **Tests only outside the project.** The acceptance tests of this lesson will not
  exist in the real project. The tests you add to its own suite are what protects
  your feature after the merge.
- **A changelog entry that describes the code.** "Added a field to RunOptions" tells
  a user nothing. "`--dry-run` shows what would run" does.

**Read more:** [SemVer compatibility in the Cargo book](https://doc.rust-lang.org/cargo/reference/semver.html) ·
[the `non_exhaustive` attribute](https://doc.rust-lang.org/reference/attributes/type_system.html#the-non_exhaustive-attribute) ·
[the `deprecated` attribute](https://doc.rust-lang.org/reference/attributes/diagnostics.html#the-deprecated-attribute) ·
[API guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)

## Lesson 05 · Code review, both sides

A review answers three questions, in this order: Is the change **correct**? Does it
meet the **project's rules**? Can the project **live with it** later? Taste comes
after all three, and it never blocks.

### As a reviewer

Read in this order: the description (what does the author claim?), the tests (what
do they prove?), then the code (does it do what is claimed, and what else?). Look at
CI. For anything that is not obvious, check out the branch and run it.

Sort every finding before you write it down:

- **Blocking:** merging this would break users or a written rule of the project: a
  wrong result, a panic that a user can trigger, a breaking API change, a test that
  was weakened until it passed, missing tests or changelog where the project
  requires them.
- **Non-blocking:** true and useful, but the merge does not depend on it. Mark it,
  for example with `nit:`, so the author knows.

The third category is the one that separates good reviewers from loud ones:
**comments that are wrong**. "This cannot compile" about code that compiles, "this
allocates" about code that does not. A wrong comment costs the author an hour and
costs you their trust. **Check a claim before you post it.** Most claims about Rust
code can be checked in a scratch file in two minutes.

### Worked example

A patch adds this function, with a test for `[2, 4]`:

```rust,should_panic
/// The mean of the values.
pub fn average(values: &[u32]) -> u32 {
    values.iter().sum::<u32>() / values.len() as u32
}

// What the reviewer tried in a scratch file:
average(&[]);
```

Three draft comments:

1. "`average(&[])` panics with a division by zero. Callers pass user input. Please
   return `Option<u32>` or document the panic and add a test." **Blocking**, and the
   reviewer ran it: the block above is that check.
2. "nit: `mean` would match the doc comment." **Non-blocking.**
3. "`sum::<u32>()` does not compile without importing `std::iter::Sum`." **Wrong.**
   The block above compiles. Trying it would have saved a comment.

A fourth finding needs thought: the sum can overflow `u32` for large inputs. Whether
that blocks depends on what the project promises about its inputs. When you are not
sure, ask a question; a question is a fine review comment.

### As an author

Every comment gets an answer, and there are only three kinds of answer:

- **Done:** you changed it. Say what you changed, so that the reviewer does not have
  to search.
- **Answered:** the reviewer asked why. Explain, and consider putting the
  explanation into the code or the description, because the next reader will ask
  too.
- **Declined:** you will not do it here. This is allowed and often right, for
  example when a request has nothing to do with the purpose of the pull request, or
  would be a breaking change. Decline politely, give the reason, and say where the
  idea should go (an existing issue, or a new one).

Whose comment it is matters. A maintainer who states a condition for the merge is
telling you the price. A passer-by who asks for an extra is making a suggestion.

### Mistakes people really make

- **Reviewing only what is easy to see.** Formatting and names are visible in the
  diff; a missing error path is not. Ask of every new function: what happens with
  empty input, missing files, the largest value?
- **Approving a weakened test.** When a test changes in the same patch as the code
  it tests, ask which of the two was wrong, and how the author knows.
- **Arguing about taste.** If the project has a rule, cite it. If it has none, mark
  your comment as a preference and move on.
- **Silence.** An unanswered comment stops a review. "I will not do this because..."
  is an answer; nothing is not.

**Read more:** [the Rust compiler team's review policy](https://forge.rust-lang.org/compiler/reviews.html) ·
[the pull request life cycle in the rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/pr-lifecycle.html)

## Lesson 06 · Writing for maintainers

Everything you write in a project is read by somebody with little time and without
your context. Three readers matter most:

- the **reviewer**, today, who decides whether to merge;
- the **maintainer who triages issues**, who decides in half a minute whether your
  report can be acted on;
- the **person who runs `git log` in two years**, hunting a bug in the lines you
  changed. Often that person is you.

### The commit message

A commit message has a subject line, a blank line, and a body.

The **subject** is what `git log --oneline` shows, so it must make sense alone. Say
what the commit does to the program, in the imperative: "keep", "reject", "add".
Many projects want an area prefix. Lengths differ between projects: 50 characters is
a common limit, tasklane allows 60. Where the issue reference goes differs too:
tasklane wants `Fixes #N` as the last line of the commit message, while the guide
for `rust-lang/rust` prefers it in the pull request description. **Follow the
project, not your habit**; lesson 06 checks tasklane's rules for that reason.

The **body** answers what the diff cannot: What did the user see? Why is this the
right fix? What did you consider and reject? It does not list files; the diff does.

Compare two messages for the same change:

```text
Fix bug

Changed cli/lib.rs and tests.
```

```text
cli: name the Lanefile in error messages

`tasklane -f ci.lane run x` printed `error: unknown task x` without
saying which file was read. With several Lanefiles in one CI job the
message did not say where to look. Prefix every Lanefile error with
the path that was given to --file.

Fixes #6
```

The first one says nothing that `git show --stat` would not say. The second one can
be found by searching the log for the message a user saw.

### The pull request description

The description is for the reviewer and is gone from daily view after the merge; the
commit message stays. So the description holds what helps a review: what changes,
why, **how you tested it** (commands the reviewer can repeat, and what you tried by
hand), and what you deliberately did not do. If the project has a template, fill in
every part. An unticked checklist item is a question the reviewer has to ask.

### The issue report

A report that can be acted on has five parts:

1. **A title that names the input and the effect.** "`$3` in a step argument becomes
   `$`", not "output wrong".
2. **Steps to reproduce:** the smallest input and the exact command, as text in a
   code block. Never a screenshot of text.
3. **What you expected**, and where the expectation comes from.
4. **What happened instead**, copied, not retold.
5. **The version**, and anything unusual about your setup.

If you already know the cause, add it under a separate heading. Keep observation and
theory apart: the observation is certain, the theory may be wrong.

### Mistakes people really make

- **A subject that names the activity:** "fix", "update", "address review comments".
  It was true of every commit ever made.
- **The reason only in the pull request.** Hosting platforms change; the git history
  stays. Put the "why" into the commit message.
- **"Latest version".** Latest when? Give the number.
- **A report that asks for a fix "asap".** Urgency belongs to you, not to volunteers.
  A good reproduction is the fastest way to a fix.

**Read more:** [`git commit`, section "Discussion"](https://git-scm.com/docs/git-commit#_discussion),
on the subject line and the blank line ·
[bug reports in the rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/contributing.html#bug-reports) ·
[what `rust-lang/rust` asks of a pull request description](https://rustc-dev-guide.rust-lang.org/pr-lifecycle.html) ·
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## Lesson 07 · Unsafe code and soundness reports

Safe Rust makes one promise: no program written only in safe code has undefined
behaviour (UB). Code in an `unsafe` block is allowed to do things the compiler
cannot check, and takes over the duty to keep that promise.

A function is **unsound** when it is safe to call but some input makes it cause UB.
The caller did nothing wrong and wrote no `unsafe`; the function's author broke the
promise for them. That is worse than an ordinary bug in two ways: it may do anything
(wrong data, a crash, or nothing visible), and it usually **passes every ordinary
test**.

### Reading a safety comment as a proof

Every unsafe operation has requirements, listed under "Safety" in its documentation.
A `// SAFETY:` comment is a claim that they hold. To review it:

1. Write down **every** requirement of every unsafe call in the block.
2. For each one, find the line of code that makes it true for **all** inputs a safe
   caller can pass.

A comment that names one requirement and stops is the typical finding.

### Worked example

```rust,no_run
/// The byte that the next `push` will overwrite, or `None` if the vector is full.
pub fn next_slot(v: &Vec<u8>) -> Option<u8> {
    if v.len() < v.capacity() {
        // SAFETY: the index is below the capacity, so the pointer is inside the
        // vector's allocation.
        Some(unsafe { *v.as_ptr().add(v.len()) })
    } else {
        None
    }
}
```

The comment proves one requirement of reading through a pointer: the pointer is in
bounds. Reading has a second one: the memory must hold an initialised value. Between
`len` and `capacity` it does not. Any safe caller reaches this with
`Vec::with_capacity(8)` and one `push`.

A test for exactly that case passes, in debug and in release:

```text
test tests::vector_with_room_has_a_next_slot ... ok
```

**Miri** is an interpreter for Rust that checks every operation against the language
rules while it runs your tests. The same test under `cargo +nightly-2026-11-20 miri test`:

```text
test tests::vector_with_room_has_a_next_slot ... error: Undefined Behavior: reading memory at alloc48046[0x1..0x2], but memory is uninitialized at [0x1..0x2], and this operation requires initialized memory
 --> src/lib.rs:6:23
  |
6 |         Some(unsafe { *v.as_ptr().add(v.len()) })
  |                       ^^^^^^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

### What Miri sees and what it does not

- Miri checks only the code that **runs**. It needs a test that reaches the bad case;
  writing that test is your part, and it is the heart of lesson 07.
- Miri checks the rules of the **language**: bounds, initialisation, aliasing,
  alignment, data races. It does not know the extra rules of **library** types. A
  `&str` must be valid UTF-8, but that is a rule of the standard library; a `&str`
  with broken UTF-8 passes Miri and misbehaves later, somewhere else. For such rules
  you need a test that checks the value.
- When your crate is built with debug assertions (the default for `cargo build` and
  `cargo test`), the standard library checks some requirements of its unsafe
  functions and aborts the process with `unsafe precondition(s) violated: ...`. A
  release build has no such check, and the message itself says that the check "is
  optional, and cannot be relied on for safety".

### Fixing

Prefer the safe function that does the same job: `get` in place of `get_unchecked`,
`str::get(a..b)` in place of raw parts. If measurements show that the safe version
is too slow, keep `unsafe`, but make the check complete and the comment a complete
proof. Either way: the test that showed the problem stays in the suite.

### Reporting

Unsound code in a released library is a security matter until shown otherwise.
Report it **privately first**, the way the project's security policy says; do not
open a public issue with a recipe. When the fix is released, an **advisory** tells
users what they need to decide whether they are affected:

- which function, in which versions;
- that **safe code** can trigger it, and how (a short program);
- the impact, stated exactly: say what you have shown, not what you can imagine;
- the fixed version, and what users can do until they upgrade.

**Read more:** [behaviour considered undefined](https://doc.rust-lang.org/reference/behavior-considered-undefined.html) ·
[`slice::from_raw_parts`, "Safety"](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html#safety) ·
[the Rustonomicon on working with unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html) ·
[Miri](https://github.com/rust-lang/miri) ·
[the Rust security policy](https://rust-lang.org/policies/security/) ·
[the RustSec advisory database](https://rustsec.org/)

## Lesson 08 · Dependencies and upgrades

`Cargo.toml` states **requirements** ("bitflags, version 2.something"). `Cargo.lock`
records the **decision** (exactly 2.13.2, with a checksum). Cargo picks one version
for all crates whose requirements are compatible with each other. Requirements that
are not compatible, such as `1.x` and `2.x`, get one copy each. That is a
**duplicated dependency**.

```text
$ cargo tree --duplicates
bitflags v1.3.2
└── tasklane-plugin v0.4.0 (fixture/crates/tasklane-plugin)
    ├── tasklane-cli v0.4.0 (fixture/crates/tasklane-cli)
    └── tasklane-core v0.4.0 (fixture/crates/tasklane-core)
        └── tasklane-cli v0.4.0 (fixture/crates/tasklane-cli)

bitflags v2.13.2
└── tasklane-cli v0.4.0 (fixture/crates/tasklane-cli)
```

The tree is printed upside down: below each version you see who asks for it. A
duplicate costs build time and binary size. It becomes a real problem when the
dependency's types cross crate borders, because the two copies define **different
types** with the same name; a value of one cannot be passed where the other is
expected.

To remove a duplicate, the crates must agree. Either the old user moves forward (an
upgrade), or, when the old user is somebody else's crate, you wait for their release
or help them make it.

### An upgrade across a breaking release

1. **Read the changelog** of the release that starts the new major version. Authors
   list there what breaks and what to do about it. Reading it takes ten minutes and
   replaces hours of guessing. A crate's page on crates.io links its repository.
2. **Change the requirement in one place.** In a workspace that place is
   `[workspace.dependencies]`; members say `bitflags.workspace = true`. Two members
   with their own version lines are how the duplicate came to exist.
3. **Build.** The compiler now lists the changes that are visible in types. After
   changing only the version in the fixture, the first errors include:

   ```text
   error[E0615]: attempted to take value of method `bits` on type `TaskFlags`
     --> crates/tasklane-plugin/src/lib.rs:69:14
      |
   69 |         self.bits.count_ones()
      |              ^^^^ method, not a field
      |
   help: use parentheses to call the method
   ```

4. **Run the tests.** Some changes compile and behave differently. With the compile
   errors fixed, the fixture's suite reports this, and no compiler would have:

   ```text
     left: "clean [taskflags(silent)]\npages - Writes the two pages.\n..."
    right: "clean [silent]\npages - Writes the two pages.\n..."
   ```

   The output of a `Debug` implementation changed. `Debug` output is for developers
   and is not a stable format; text that users see must not be built from it.
5. **Check your own public API.** If a type from the dependency, or a type generated
   by its macro, is part of *your* public API, then the dependency's breaking change
   may be yours too. Your users may compare, hash or print that type, or call a
   method that the new version no longer generates. Your own crate never did these
   things, so its build and its tests stay green. Everything users could do before
   must still work; a crate like `downstream/pinned` is how you find out. When
   something is gone for good, put it back by hand with `#[deprecated]` and remove
   it in your own next breaking release.
6. **Commit the lock file with the change**, and write a changelog entry: your users
   see the new version in their own trees.

One more thing the compiler told us while this lesson was written: a new major
version can add methods to its generated types, and one of them can have the name of
a method you wrote yourself.

```text
error[E0592]: duplicate definitions with name `from_name`
```

### Mistakes people really make

- **Expecting `cargo update` to do it.** `cargo update` moves to newer *compatible*
  versions inside the requirements. It never crosses a breaking version, and with
  exact requirements (`=1.3.2`) it has nothing to move at all. A breaking upgrade
  starts with an edit of `Cargo.toml`.
- **Stopping when it compiles.** See step 4.
- **Upgrading everything at once.** One dependency per pull request; when something
  breaks, you know which one.
- **Deriving fewer traits than before** on a public type because "nothing in this
  repository needs `Hash`". Somebody else's repository does.

### When to use which

| Situation | Tool |
| --- | --- |
| see duplicates | `cargo tree --duplicates` |
| who pulls this crate in? | `cargo tree --invert NAME@VERSION` |
| newer compatible version, same requirement | `cargo update NAME`, or `cargo update NAME --precise VERSION` |
| newer breaking version | edit the requirement, then follow the six steps |
| keep members in step | `[workspace.dependencies]` |

**Read more:** [specifying dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) ·
[how the resolver chooses versions](https://doc.rust-lang.org/cargo/reference/resolver.html) ·
[`cargo tree`](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) ·
[`[workspace.dependencies]`](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-dependencies-table) ·
[`cargo update`](https://doc.rust-lang.org/cargo/commands/cargo-update.html)

## Lesson 09 · Contributing to the wider ecosystem

The Rust project is run in the open, and its rules are written down. This section is
a map of where they are written, as they stood in **November 2026**, the time of the
pinned toolchain. Processes change. When this text and a linked page disagree, the
page is right.

### Where the rules are

| Document | Covers |
| --- | --- |
| [rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/) | working on the compiler: building, testing, the life of a pull request |
| [std-dev-guide](https://std-dev-guide.rust-lang.org/) | working on the standard library |
| [Rust Forge](https://forge.rust-lang.org/) | procedures of the teams, the bots, the policies |
| [RFC repository](https://github.com/rust-lang/rfcs) | how substantial changes are proposed and decided |
| [Edition Guide](https://doc.rust-lang.org/edition-guide/) | what editions are and what each one changed |

### Small changes

Bug fixes, tests, documentation and better diagnostics are ordinary pull requests
to [rust-lang/rust](https://github.com/rust-lang/rust). A bot assigns a reviewer;
`r? @name` asks for a specific one. Labels show whose turn it is:
`S-waiting-on-review` or `S-waiting-on-author`. After you have pushed what the
reviewer asked for, the comment `@rustbot ready` puts the pull request back into the
queue. Update your branch by **rebasing**, not by merging. Reviewers are mostly
volunteers; the guide suggests waiting two weeks before you ask for attention.

### Substantial changes

- **Language and other large changes** need an **RFC** (request for comments): a
  design document sent as a pull request to the RFC repository. The responsible
  team discusses it. When the discussion has settled, a team member proposes a
  **final comment period (FCP)** with a disposition: merge, close or postpone. The
  FCP lasts ten calendar days. A merged RFC is "active": it may be implemented, which
  is not a promise that it will be.
- **Implementation** happens on nightly behind a **feature gate**, with a **tracking
  issue** that collects open questions.
- **Stabilisation** is a pull request with a stabilisation report; the team again
  goes through an FCP. After the merge the feature reaches stable with the normal
  release train.
- **New standard-library API** starts lighter: an **API change proposal (ACP)**, an
  issue in the `rust-lang/libs-team` repository. Then an unstable implementation, a
  tracking issue, and an FCP before stabilisation. The bar is high for a reason the
  std-dev-guide states plainly: the standard library is not versioned, so a stable
  API can never be removed.
- **Changes that might break existing code** are measured with **crater**, a tool
  that builds and tests the crates on crates.io with the changed compiler.
- **Editions** (2015, 2018, 2021, 2024) are how the language makes incompatible
  changes at all. A crate chooses its edition, crates of different editions work
  together, and `cargo fix --edition` does most of a migration.

### Finding work, and manners

The guide's [issue search](https://rustc-dev-guide.rust-lang.org/getting-started.html#what-should-i-work-on)
uses the labels `E-easy`, `E-medium`, `E-help-wanted` and `E-mentor`. Two labels are
especially good for a first contribution: `E-needs-test` (the bug is fixed, the
regression test is missing) and `A-diagnostics` (an error message can be better).

- Comment `@rustbot claim` to assign an issue to yourself, and
  `@rustbot release-assignment` when you stop. Claim one issue, not five.
- Ask questions in the open. The project's Zulip chat is the main place; for the
  compiler, the Forge names the channel `t-compiler/help`. A specific question
  with what you tried gets answered.
- Do not send pull requests that only fix spelling in code comments or internal
  documentation of the compiler; its guide says they are not accepted, because the
  churn and the review time cost more than the fix gives.

### Licences, conduct, and tools that write for you

- **Licence.** Rust is dual-licensed under the MIT licence and the Apache
  License 2.0. Its [COPYRIGHT file](https://github.com/rust-lang/rust/blob/main/COPYRIGHT)
  says that contributors keep their copyright and that no copyright assignment is
  required. Other projects ask for more: a contributor licence agreement (CLA),
  signed once, or the [Developer Certificate of Origin](https://developercertificate.org/),
  a statement you make per commit with `git commit -s`. A project's CONTRIBUTING
  file says which applies. Also check that you may contribute code you wrote at
  work.
- **Conduct.** The [code of conduct](https://rust-lang.org/policies/code-of-conduct/)
  applies in every project space.
- **LLMs.** `rust-lang/rust` and some other repositories of the project (Cargo,
  Clippy, rustfmt, mdBook, rustlings) follow an
  [LLM usage policy](https://forge.rust-lang.org/policies/llm-usage.html), announced
  in August 2026. In short: private use, where only you see the output, is allowed.
  Comments, issue texts, pull request descriptions, documentation and diagnostics
  created by an LLM are banned. Some uses are allowed only **with disclosure**:
  trivial changes such as a typo fix, machine translation, and reporting a bug that
  an LLM pointed out and you verified yourself. Code created by an LLM is allowed
  only inside a described experiment: arranged with a reviewer beforehand and
  disclosed. Read the page before you contribute; it is exact and expected to
  change. Other projects have other rules. Where a project has none, disclose
  anyway.

### LAB: build Clippy from source and run its tests

Clippy is a good first look inside the project: it is a normal Cargo workspace, it
builds in about a minute, and most of its tests are small Rust files.

**Before you start:** change into a directory **outside the course repository**, for
example a scratch directory in your home. The lab clones a repository; a clone
inside the course repository would show up in its `git status`. The first `cargo`
command in the clone makes rustup download the nightly toolchain that Clippy pins
(about 1 GB, into your rustup directory). You need about 8 GB of free disk space.

```sh
git clone --depth 1 https://github.com/rust-lang/rust-clippy.git
cd rust-clippy
git rev-parse HEAD             # the commit you have; note it for the lab notes
cat rust-toolchain.toml        # the nightly this commit needs; rustup installs it
cargo build                    # builds Clippy
cargo test                     # all tests
TESTNAME="needless_return" cargo uitest    # only the UI tests with this name
```

Expected: `cargo build` ends with `Finished`, and every test binary of `cargo test`
ends with `test result: ok.`. The filtered run prints lines such as
`tests/ui/needless_return.rs ... ok` and a first result line like
`test result: ok. 4 passed; 1237 filtered out`; your numbers will differ. A warning
from the linker during the build is harmless.

For this course the lab was run on commit `ddecc859` (17 November 2026), which pins
`nightly-2026-11-17`, on one machine: the build took under a minute and the full
test run about a minute and a half; the toolchain took 1.1 GB and the build
directory 6 GB.

If rustup does not install the toolchain by itself (the automatic installation can
be switched off in rustup's settings), run `rustup toolchain install` inside the
clone: without a name, it installs the toolchain that the directory asks for.

Self-check: open `tests/ui/needless_return.rs` next to `needless_return.stderr`,
the expected compiler output for it. Find the lint's implementation under
`clippy_lints/src/`. Record what you saw in `answers/l09_lab.md`.
`cargo dev --help` lists the project's own tools.

Clean up when you are done: delete the `rust-clippy` directory, and remove the
toolchain with `rustup toolchain uninstall nightly-YYYY-MM-DD`, using the date from
Clippy's `rust-toolchain.toml`. `rustup toolchain list` shows what is installed.

### A path to a first real contribution

1. Choose a project whose output you read every week: the compiler's diagnostics,
   Clippy, a crate you depend on.
2. Do the lab for that project: clone, build, run the tests. Read its CONTRIBUTING
   file completely.
3. Choose something small that needs no design decision: a regression test for an
   `E-needs-test` issue, a clearer diagnostic for a case you hit yourself, a Clippy
   issue labelled `good first issue`, a missing example in documentation.
4. Say in the issue that you are working on it (in rust-lang repositories:
   `@rustbot claim`).
5. Work as in lessons 02, 04 and 06. In Clippy, the pull request description needs a
   `changelog:` line.
6. Answer the review as in lesson 05. Expect it to take days, and be kind.

**Read more:** [the RFC process](https://github.com/rust-lang/rfcs#what-the-process-is) ·
[the std feature life cycle](https://std-dev-guide.rust-lang.org/development/feature-lifecycle.html) ·
[stabilising a language feature](https://rustc-dev-guide.rust-lang.org/stabilization-guide.html) ·
[crater](https://rustc-dev-guide.rust-lang.org/tests/crater.html) ·
[issue assignment](https://forge.rust-lang.org/triagebot/issue-assignment.html) and
[shortcuts](https://forge.rust-lang.org/triagebot/shortcuts.html) of the bot ·
[Clippy: basics for hacking](https://doc.rust-lang.org/clippy/development/basics.html) ·
[Clippy's CONTRIBUTING](https://github.com/rust-lang/rust-clippy/blob/main/CONTRIBUTING.md) ·
[how to start contributing, on the Forge](https://forge.rust-lang.org/how-to-start-contributing.html)

## Capstone · One contribution, from issue to merge

The capstone has no new idea. It asks you to do the whole path once, without the
lessons cutting it into steps for you. Choose **one** of the three open issues:

| Issue | Kind | The hard part |
| --- | --- | --- |
| 22: `plan` never finishes | performance regression | a reproduction that fails fast, and a fix that keeps the order and the cycle errors |
| 25: `concat` skips files | wrong result | the algorithm, and not matching more than before |
| 30: a test fails every other run | flaky test | finding the cause in the code, not in the test |

Work in this order, and do not skip a step because the fix looks obvious:

1. **Read the issue and its comments.** The maintainers say what must not change.
2. **Reproduce** it with the fixture's command or API, exactly as reported.
3. **Reduce** it to a failing test inside the fixture's own suite. Run it and watch
   it fail for the reported reason. For issue 22, a test that "fails" by never
   finishing is not usable, and the test must stay in the suite afterwards. Choose a
   size that a correct planner finishes in milliseconds and the current one cannot
   finish at all, and give the test a time limit: plan on a helper thread
   (`std::thread::spawn`), send the result through a channel, and wait for it with
   `recv_timeout`. Then the test fails after a few seconds, with a message. For
   issue 30, a test that fails every other run is not enough either: make one that
   fails on every run.
4. **Find the cause.** State it in one sentence that names a function.
5. **Fix** the cause with the smallest change.
6. **Check what must not change:** the fixture's whole suite, clippy, the
   `unchanged` tests of your issue, and `--test downstream`.
7. **Write** the changelog entry, the commit message and the pull request.
8. **Grade yourself** with the rubric in the [README](README.md#capstone-rubric).

A note on issue 30. A flaky test tempts people to change the test: accept both
orders, sort in the test, add `#[ignore]`. Ask first what the test is telling you. A
test that fails half of the time is a test that has found something half of the
time.

**Read more:** the sections above, and `fixture/CONTRIBUTING.md`: in the capstone
its merge requirements are your checklist.
