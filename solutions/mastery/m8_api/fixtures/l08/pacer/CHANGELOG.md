# Changelog

All notable changes to `pacer` are listed here, newest first. The version
numbers follow [Semantic Versioning](https://semver.org/).

## 1.1.0

A minor release: everything that compiled with 1.0.0 still compiles and
behaves as before.

### Added

- `Policy`: a retry policy with a growth factor (`with_factor`) and a largest
  delay (`with_cap_ms`). Build one with `Policy::new`, or from a `Config` with
  `Policy::try_from`.
- `Delays`: the iterator that `Policy::delays` returns.
- `PolicyError` and `PolicyErrorKind`: why a policy cannot be built.
- `total_ms`: the new name of `get_total_ms`.
- `Clock::elapsed_since`: a provided method, so existing implementations of
  `Clock` need no change. If a type of yours has a method of this name through
  another trait, call that trait's method as `Trait::elapsed_since(&value, ..)`.

### Deprecated

- `get_total_ms`: use `total_ms` instead. The old name keeps working in every
  1.x release.

## 1.0.0

### Added

- `Config`, `delays`, `get_total_ms`, `Clock`, `deadline` and `Error`.
