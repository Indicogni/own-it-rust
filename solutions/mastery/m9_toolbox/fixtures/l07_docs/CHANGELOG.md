# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the version numbers
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-10-05

### Added

- `title_case`, behind the new `titles` feature.

### Fixed

- Several separators in a row gave several dashes: `"a -- b"` became `"a--b"`.
  It now becomes `"a-b"`.

## [0.1.0] - 2026-08-17

### Added

- `slugify` and `Slug`.
