# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.8](https://github.com/gadomski/las-rs/compare/v0.9.7...v0.9.8) - 2025-12-15

### Added

- crs ([#130](https://github.com/gadomski/las-rs/pull/130))

### Other

- remove doc_auto_cfg for docs.rs ([#132](https://github.com/gadomski/las-rs/pull/132))
- environment for release-plz

## [0.9.7](https://github.com/gadomski/las-rs/compare/v0.9.6...v0.9.7) - 2025-12-15

### Other

- add crates.io environment
- add release-plz ([#127](https://github.com/gadomski/las-rs/pull/127))
- add reproducer for https://github.com/gadomski/las-rs/issues/15 ([#126](https://github.com/gadomski/las-rs/pull/126))
- *(deps)* update criterion requirement from 0.7 to 0.8 ([#123](https://github.com/gadomski/las-rs/pull/123))
- *(deps)* bump actions/checkout from 5 to 6 ([#122](https://github.com/gadomski/las-rs/pull/122))
- *(deps)* update laz requirement from 0.10.0 to 0.11.0 ([#119](https://github.com/gadomski/las-rs/pull/119))

### Added

- `Sync` restriction for reading and writing ([#119](https://github.com/gadomski/las-rs/pull/119))

### Removed

- `Drop` implementation for `Writer` ([#119](https://github.com/gadomski/las-rs/pull/119))

## [0.9.6] - 2025-09-04

### Added

- `ReaderOption` ([#117](https://github.com/gadomski/las-rs/pull/104))

### Fixed

- Made a couple more fields public for COPC support ([#110](https://github.com/gadomski/las-rs/pull/110))

## [0.9.5] - 2025-04-21

### Added

- COPC support ([#104](https://github.com/gadomski/las-rs/pull/104))

## [0.9.4] - 2025-04-08

### Changed

- Refactor to remove some duplicate code ([#101](https://github.com/gadomski/las-rs/pull/101))
- Edition 2024 ([#102](https://github.com/gadomski/las-rs/pull/102))

## [0.9.3] - 2024-12-17

### Fixed

- Ignore everything after first null character ([#100](https://github.com/gadomski/las-rs/pull/100))

## [0.9.2] - 2024-11-06

### Fixed

- Relax string parsing ([#98](https://github.com/gadomski/las-rs/pull/98))

## [0.9.1] - 2024-08-22

### Fixed

- Version numbers in documentation

## [0.9.0] - 2024-08-22

### Added

- Several methods in `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
  - `Reader::read_point`, which returns a `Result<Option<Point>>`
  - `Reader::read_points`
  - `Reader::read_points_into`
  - `Reader::read_all_points_into`
- `Writer::write_point`, `Header::write_to`, `laz` module, a few laz-specific methods on `Header` ([#90](https://github.com/gadomski/las-rs/pull/90))

### Changed

- Reorganize reading, including removing the lifetime specifier on `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
- Conslidate errors to a single enum ([#87](https://github.com/gadomski/las-rs/pull/87))

### Fixed

- Start of first EVLR ([#91](https://github.com/gadomski/las-rs/pull/91))

## Deprecated

- `Read` trait ([#88](https://github.com/gadomski/las-rs/pull/88))
- `Write` trait ([#90](https://github.com/gadomski/las-rs/pull/90))
- Many methods on `Reader` ([#89](https://github.com/gadomski/las-rs/pull/89))
  - `read` in favor of `read_point`
  - `read_n` in favor of `read_points`
  - `read_n_into` in favor of `read_points_into`
  - `read_all_points` in favor of `read_all_points_into`
- `Writer::write` ([#90](https://github.com/gadomski/las-rs/pull/90))

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

[Unreleased]: https://github.com/gadomski/las-rs/compare/v0.9.6...HEAD
[0.9.6]: https://github.com/gadomski/las-rs/releases/compare/v0.9.5...v0.9.6
[0.9.5]: https://github.com/gadomski/las-rs/releases/compare/v0.9.4...v0.9.5
[0.9.4]: https://github.com/gadomski/las-rs/releases/compare/v0.9.3...v0.9.4
[0.9.3]: https://github.com/gadomski/las-rs/releases/compare/v0.9.2...v0.9.3
[0.9.2]: https://github.com/gadomski/las-rs/releases/compare/v0.9.1...v0.9.2
[0.9.1]: https://github.com/gadomski/las-rs/releases/compare/v0.9.0...v0.9.1
[0.9.0]: https://github.com/gadomski/las-rs/releases/compare/v0.8.8...v0.9.0
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
