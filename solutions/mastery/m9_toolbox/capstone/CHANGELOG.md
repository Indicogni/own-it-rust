# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.4.0] - 2026-11-20

### Added

- The feature `json`; JSON output can now be left out of a build.
- Tests, including snapshot tests of the report, and a fuzz target for the parser.

### Changed

- `parse_line` and `parse` return a `Result` instead of panicking on bad input.
- The report lists categories in alphabetical order, with aligned columns.

### Fixed

- Sums that do not fit in 64 bits are now exact instead of overflowing.
- `12.5` was read as 12.05. It now means 12.50.
- An amount without a decimal point (`1200`) crashed the program.
