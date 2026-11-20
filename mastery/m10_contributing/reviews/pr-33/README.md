# Pull request 33: "Add a sort step, clean up lookup, speed up globs"

This directory is a pull request against tasklane 0.4.0 (the fixture as shipped),
waiting for your review in lesson 05.

| File | What it is |
| --- | --- |
| `0001-…patch` to `0004-…patch` | The four commits, as `git format-patch` wrote them. Read them in order. |
| `ci.txt` | What the project's CI printed for the branch. |
| `candidate-comments.md` | Review comments that other people drafted. Task B asks you to judge them. |

The patches are for reading. They apply cleanly to the fixture as shipped; after
your changes from other lessons they may not, and you do not need to apply them.

## The description the author wrote

> **Add a sort step, clean up lookup, speed up globs** (#33)
>
> Hi! First contribution here. I needed a `sort` step for my contributor list, and
> while I was in the code I fixed a few other things I noticed:
>
> - new step: `sort PATH`
> - the flaky summary test (#30) is fixed
> - `Registry::lookup` now takes `&str` (#27)
> - `glob::matches` no longer allocates
>
> Tests pass on my machine.
>
> — Sam

## How to review

1. Read `fixture/CONTRIBUTING.md` again: it says what this project requires before a
   merge. A review judges a change against the project's rules, not against your
   personal taste.
2. Read the description, then each patch. For each one ask: What does it claim? Does
   the code do that? What else does it change? What can go wrong for a user?
3. Look at `ci.txt`.
4. Write your own findings in `NOTES.md` **before** you open `candidate-comments.md`.
