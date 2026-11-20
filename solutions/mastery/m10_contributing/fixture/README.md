# tasklane

tasklane runs named tasks in dependency order. You describe the tasks in a
*Lanefile*; tasklane works out the order, runs each task once, and tells you what
happened.

A task is a list of *steps*, and a step is a small operation on text files: write a
file, join files, convert to upper case. All files live in an in-memory *workspace*,
so a run never touches your disk and gives the same result every time.

```text
$ tasklane -f examples/site.lane run site --show log,files,summary
[header] write build/pages/00-header.txt == My Site ==
[pages] write build/pages/01-home.txt Welcome to My Site.
[pages] write build/pages/02-about.txt Made with tasklane.
build/all.txt: 3 lines, 11 words
[site] concat build/site.txt build/pages/*.txt
file build/pages/00-header.txt (13 bytes)
file build/pages/01-home.txt (19 bytes)
file build/pages/02-about.txt (19 bytes)
file build/site.txt (53 bytes)
5 done, 0 failed, 0 skipped
```

## The Lanefile

```text
# A comment.
set OUT = build

## Shown by `tasklane list`.
task pages : clean
  flags silent
  step write ${OUT}/index.txt "Hello, world"
```

- `set NAME = VALUE` defines a variable. `${NAME}` in a step argument is replaced by
  its value; `$$` gives a literal `$`.
- `task NAME : DEP DEP` starts a task and names the tasks that must run first.
- Indented `step NAME ARGS...` lines are the task's steps. Double quotes keep spaces
  inside one argument.
- Indented `flags` lines set switches: `silent` (do not log the steps), `optional`
  (a failure does not stop the tasks that depend on this one), `hidden` (not shown by
  `tasklane list`).

`tasklane steps` lists the built-in steps.

## The command

```text
tasklane [OPTIONS] COMMAND
```

| Command | What it does |
| --- | --- |
| `list` | shows the tasks of the Lanefile |
| `plan TASK` | shows what `run TASK` would run, in order |
| `run TASK` | runs a task and everything it depends on |
| `check` | parses the Lanefile and plans every task |
| `steps` | shows the available steps |

| Option | Meaning |
| --- | --- |
| `-f`, `--file PATH` | the Lanefile to read (default: `Lanefile`) |
| `--set NAME=VALUE` | set or override a variable; may be repeated |
| `-k`, `--keep-going` | after a failure, still run tasks that do not depend on it |
| `-n`, `--dry-run` | check the run and log what would run, but run nothing |
| `--show PARTS` | what `run` prints: `log`, `files`, `summary`, separated by commas |
| `-q`, `--quiet` | the same as `--show summary` |

Exit codes: 0 success, 1 a task failed, 2 wrong command line, 3 a mistake in the
Lanefile, 4 the Lanefile could not be read.

## The crates

| Crate | Contents |
| --- | --- |
| `crates/tasklane-plugin` | the `Step` trait and the `Workspace`: all a plugin needs |
| `crates/tasklane-core` | parser, planner, runner, built-in steps |
| `crates/tasklane-cli` | the `tasklane` command |

To add your own step, implement `tasklane_plugin::Step` and register it:

```rust,ignore
let mut registry = tasklane_core::Registry::with_builtins();
registry.register(Box::new(MyStep))?;
```

The documentation of `tasklane-plugin` has a complete example. Build all of it with
`cargo doc --workspace --no-deps --open`.

## Contributing

Bug reports and pull requests are welcome. Please read
[CONTRIBUTING.md](CONTRIBUTING.md) first; the open issues are in [issues/](issues/).

## Licence

MIT.
