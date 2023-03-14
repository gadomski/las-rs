# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1] - 2023-03-14

### Fixed

- Possible panic when reading invalid laz files ([#58](https://github.com/gadomski/las-rs/pull/58))

## [0.8.0] - 2022-11-30

### Added

- This CHANGELOG ([#53](https://github.com/gadomski/las-rs/pull/53))

### Changed

- `Builder::date` is now a `NaiveDate`, instead of a `Date<Utc>` ([#52](https://github.com/gadomski/las-rs/pull/52))
- Benchmarks now use [criterion](https://github.com/bheisler/criterion.rs) ([#52](https://github.com/gadomski/las-rs/pull/52))
- Edition is now 2021 ([#52](https://github.com/gadomski/las-rs/pull/52))

[Unreleased]: https://github.com/gadomski/las-rs/compare/v0.8.1...HEAD
[0.8.1]: https://github.com/gadomski/las-rs/releases/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/gadomski/las-rs/releases/compare/v0.7.8...v0.8.0
