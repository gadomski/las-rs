# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Several methods in `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
  - `Reader::read_point`, which returns a `Result<Option<Point>>`
  - `Reader::read_points`
  - `Reader::read_points_into`
  - `Reader::read_all_points_into`

### Changed

- Reorganize reading, including removing the lifetime specifier on `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
- Conslidate errors to a single enum ([#87](https://github.com/gadomski/las-rs/pull/87))

## Deprecated

- `Read` trait ([#88](https://github.com/gadomski/las-rs/pull/88))
- Many methods on `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
  - `read` in favor of `read_point`
  - `read_n` in favor of `read_points`
  - `read_n_into` in favor of `read_points_into`
  - `read_all_points` in favor of `read_all_points_into`

## [0.8.8] - 2024-05-30

### Added

- `Builder::minimum_supported_version` ([#83](https://github.com/gadomski/las-rs/pull/83))

### Changed

- `Reader` now upgrades the las version rather than erroring when a certain feature or format is not supported ([#83](https://github.com/gadomski/las-rs/pull/83))

## [0.8.7] - 2024-05-13

### Fixed

- Deny more things ([#78](https://github.com/gadomski/las-rs/pull/78))
- Bounds calculation for negative values ([#77](https://github.com/gadomski/las-rs/pull/77))

## [0.8.6] - 2024-05-06

### Fixed

- EVLR offset for laz ([#76](https://github.com/gadomski/las-rs/pull/76))

## [0.8.5] - 2024-04-07

### Fixed

- Allow zero GPS Time values ([#75](https://github.com/gadomski/las-rs/pull/75))

## [0.8.4] - 2024-04-04

### Added

- `laz-parallel` feature ([#70](https://github.com/gadomski/las-rs/pull/70))

## [0.8.3] - 2024-03-25

### Added

- Interface for reading many points ([#68](https://github.com/gadomski/las-rs/pull/68))

## [0.8.2] - 2024-03-12

### Fixed

- WKT CRSes for all point formats ([#67](https://github.com/gadomski/las-rs/pull/67))

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

[Unreleased]: https://github.com/gadomski/las-rs/compare/v0.8.8...HEAD
[0.8.8]: https://github.com/gadomski/las-rs/releases/compare/v0.8.7...v0.8.8
[0.8.7]: https://github.com/gadomski/las-rs/releases/compare/v0.8.6...v0.8.7
[0.8.6]: https://github.com/gadomski/las-rs/releases/compare/v0.8.5...v0.8.6
[0.8.5]: https://github.com/gadomski/las-rs/releases/compare/v0.8.4...v0.8.5
[0.8.4]: https://github.com/gadomski/las-rs/releases/compare/v0.8.3...v0.8.4
[0.8.3]: https://github.com/gadomski/las-rs/releases/compare/v0.8.2...v0.8.3
[0.8.2]: https://github.com/gadomski/las-rs/releases/compare/v0.8.1...v0.8.2
[0.8.1]: https://github.com/gadomski/las-rs/releases/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/gadomski/las-rs/releases/compare/v0.7.8...v0.8.0

<!-- markdownlint-disable-file MD024 -->
