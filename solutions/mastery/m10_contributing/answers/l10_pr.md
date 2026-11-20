<!-- Capstone: the description of the pull request for the issue you chose.
     The template is from fixture/CONTRIBUTING.md, section "Pull requests". -->

# plan: walk each task once

## What

`plan` returns early for a task that is already in the plan. Before, it walked the
task's dependencies again each time the task was reached. The change is one early
return in `visit` in `crates/tasklane-core/src/graph.rs`, a regression test, and a
changelog entry.

## Why

Fixes #22. With the generated Lanefile from the issue (every stage depends on both
tasks of the stage before), the number of visits doubles with every stage: 2^47
visits of the first stage for the 48 stages in the report, which never finishes. The
regression came with the rewrite of `visit` for the cycle path in 0.4.0 (#15), which
moved the "already planned" check from the start of the function to its end.

Returning early is safe for cycle detection: a task enters the plan only after all of
its dependencies were walked without finding a cycle, so a second walk cannot find
one either. The plan order and the cycle paths are the same as before; the existing
tests for both pass unchanged.

## Testing

- `cargo test --workspace` passes, including the new
  `a_ladder_of_diamonds_is_planned_quickly`, which plans 60 levels (121 tasks) on
  a helper thread. Without the fix it fails after five seconds with "planning 121
  tasks takes milliseconds, not seconds".
- By hand: `tasklane -f ladder.lane plan l48` with the file from the issue now prints
  97 lines at once. Before the fix I stopped it after two minutes.
- I also checked a cycle that is reached after a finished dependency
  (`task a : b c`, `task b`, `task c : a`): still `dependency cycle: a -> c -> a`.
- `cargo clippy --workspace --all-targets` prints no warning.

## Checklist

- [x] Tests added or updated
- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace --all-targets` is clean
- [x] `CHANGELOG.md` updated, or not needed (say why)
