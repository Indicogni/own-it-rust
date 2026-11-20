# Changelog

All changes that users of tasklane can see are listed here, newest first. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Until 1.0, a
breaking change raises the minor version (0.4 to 0.5) and everything else raises the
patch version (0.4.0 to 0.4.1).

## [Unreleased]

### Added

- `RunOptions::dry_run` and the `-n`/`--dry-run` option: check a run and log what
  would run, without running a step (#21).

### Changed

- The library crates use bitflags 2, like the command line. `TaskFlags` keeps every
  trait and method it had; the output of its `Debug` implementation has changed
  (#19).

### Deprecated

- `TaskFlags::from_bits_unchecked`: bitflags 2 no longer generates it. It is kept
  for users of 0.4.0 and will be removed in 0.5; use the safe `from_bits_retain`
  (#19).

### Fixed

- `${...}` expansion dropped the character after a `$` that starts neither `${` nor
  `$$`: `$3` became `$` (#14).
- `plan` and `run` did not finish for targets whose dependencies form a ladder of
  diamonds (#22).
- `*` in a path pattern never gave back characters, so `*.md` did not match
  `notes.md.md` (#25).
- The summary line listed failed tasks in a different order on every run; it now
  uses plan order, as documented (#30).

### Security

- `Workspace::read_range` returned a `&str` that could reach outside the file, or
  split a character, for ranges that are not inside the file. It now returns `None`
  for them. The `slice` step was affected. See the advisory for 0.4.1.

## [0.4.0] - 2026-10-12

### Added

- The `slice` step and `Workspace::read_range` (#16).
- The `optional` flag (#17).
- `--show` and `--quiet` (#18).

### Changed

- `plan` reports the whole path of a dependency cycle, not only one task of it (#15).
- The command line uses bitflags 2 (#19, first part).

## [0.3.0] - 2026-07-06

### Added

- The plugin interface is a crate of its own, `tasklane-plugin` (#11).
- `Registry::register` for plugin steps (#11).
- The `hidden` flag (#12).

### Changed

- **Breaking:** `Runner::run` takes the variables as an argument, so that the
  command line can override them (#9).

## [0.2.0] - 2026-03-07

### Added

- Quoted arguments, the `flags` directive, comments.

### Fixed

- An unterminated `${` is an error instead of being copied to the output.

## [0.1.0] - 2026-03-03

- Tasks, dependencies, steps, `plan`.
