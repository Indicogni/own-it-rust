# Replies to the review of pull request 34

One section per comment of `reviews/on-your-change.md`. For each comment, choose
one action and write the reply you would post.

- `done`: you changed the code as asked. Say what you changed.
- `answered`: no code change was asked for. Give the explanation.
- `declined`: you will not do it in this pull request. Say why, politely, and say
  where the idea should go instead.

## Comment 1

Action: done

Reply: Done in the latest commit. A dry run now logs `[TASK] would run: STEP ARGS...`
for every step, with expanded arguments and in plan order, and logs nothing for
`silent` tasks. The line is built by the same helper as the line of a real run, so
the two cannot drift apart. `a_dry_run_changes_no_file_and_skips_every_task` checks
the exact log, including a silent task in the middle of the plan.

## Comment 2

Action: answered

Reply: A fifth parameter on `Runner::run` would be a breaking change: every existing
call has four arguments and would stop compiling, and 0.4.x must stay compatible.
`RunOptions` is `#[non_exhaustive]` with private fields, which is the project's way
to add settings without breaking anyone, and `keep_going` already works like this.
I have added this reasoning to the pull request description.

## Comment 3

Action: declined

Reply: I agree that the signature is awkward, but I would like to keep it out of this
pull request. Changing the return type of `lookup` breaks plugins that name the old
type, so it cannot go into 0.4.x at all, and it has nothing to do with the dry run.
The discussion about how to do it with a deprecation is in #27; your vote and use
case would be welcome there.

## Comment 4

Action: done

Reply: Added `dry_run_with_keep_going_still_runs_nothing` in
`crates/tasklane-core/tests/run.rs`. It sets both options on a plan with a failing
task and checks that no file is written and that every task is reported as skipped.
