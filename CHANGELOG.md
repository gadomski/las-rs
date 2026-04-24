# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

This release reshapes the public API around a new `PointData` type for
byte-slab reads and writes. `Reader` becomes a factory that produces
`PointData`; `PointData` is the container everything else operates on.
Several breaking changes are involved — see below.

### Added

- `PointData`: a byte-slab point cloud holding a single contiguous
  `Vec<u8>` of decompressed LAS records alongside the format and
  coordinate transforms needed to decode them. Supports row iteration
  as owned `Point` values via `PointData::points()` and column accessors
  (`x`, `y`, `z`, `intensity`, `classification`, `rgb`, `nir`, …) that
  sweep a single field without materializing a `Point`. Constructed by
  `Reader::read_points(n)`, `Reader::read_all()`,
  `Reader::fill_points(n, &mut pd)` for buffer reuse,
  `PointData::from_raw_bytes(...)` to wrap an existing buffer, or
  `PointData::new(...)` + `PointData::resize_for(n)` for callers driving
  their own decompressor (e.g. COPC). Addresses the LAZ decompression
  throughput gap reported in
  [#121](https://github.com/gadomski/las-rs/issues/121) — on a 42.7M-point
  (159 MB) LAZ file with `laz-parallel`, reading every point and summing
  x+y+z via column iterators runs ~2.08× faster than the equivalent
  per-`Point` loop.
- `PointData::from_points(&[Point], format, transforms)`: encodes an
  existing `Vec<Point>` into a byte slab for bulk writing.
- `Writer::write_points(&PointData)`: single-call bulk write of a byte
  slab, skipping per-`Point` decode/encode round-trips. LAS-to-LAS
  conversions become byte copies; LAZ involvement on either end uses
  `compress_many` / `decompress_many` directly.
- `Reader::read_all()`: convenience for reading every remaining point
  into a fresh `PointData`.
- `Reader::fill_points(n, &mut PointData)`: buffer-reuse fill for
  chunked streaming over files that don't fit in memory.
- `Header::add_point_data(&PointData)`: updates header stats (count,
  per-return counts, bounds) from column scans without materializing any
  `Point`.

### Changed (breaking)

- Per-`Point` iteration has moved from `Reader` to `PointData`. Load a
  slab via `Reader::read_all()` (or `Reader::read_points(n)` /
  `Reader::fill_points` for bounded memory) and iterate with
  `PointData::points()`, which yields owned `Point` values through the
  same `raw::Point::read_from` + `Point::new` pipeline the old
  `Reader::points()` used.
- `Reader::read_points(n)` now returns `Result<PointData>` (previously
  `Result<Vec<Point>>`).
- `Writer::write_points` now takes `&PointData` instead of `&[Point]`,
  mirroring `Reader::read_points`. For callers building points
  programmatically, use `PointData::from_points` to construct a slab
  first, or loop over `Writer::write_point`.

### Removed (breaking)

- `Reader::read_point()`, `Reader::points()`, and the `PointIterator`
  type — replaced by the `PointData::points()` flow described above.
- `Reader::read_points_into(n, &mut Vec<Point>)`: use
  `Reader::read_points(n)`, which now returns a `PointData`.
- `Reader::read_all_points_into(&mut Vec<Point>)`: use `Reader::read_all()`.
- Deprecated `las::Read` and `las::Write` traits plus their
  `Reader::{read, read_n, read_n_into, read_all_points}` /
  `Writer::write` inherent methods (deprecated since 0.9.0).

### Fixed

- Point format 10 on-disk field order: the old `raw::Point::read_from`
  expected `gps_time → color → waveform → nir`, while `raw::Point::write_to`
  and `Format::len()` used the LAS 1.4 spec order of
  `gps_time → color → nir → waveform`. The three paths have been unified
  through a single schema (`raw::point::fields`) and now all follow the spec
  order. Format-10 files written by older versions of `las-rs` will
  round-trip correctly through those same versions, but will be read
  incorrectly by this version; files written by spec-compliant writers
  (PDAL, LAStools, `laszip`) were always correct and are now read correctly.
  Formats 0–9 are unaffected.

## [0.9.11](https://github.com/gadomski/las-rs/compare/v0.9.10...v0.9.11) - 2026-04-07

### Fixed

- bump fmt required version ([#140](https://github.com/gadomski/las-rs/pull/140))

### Other

- Added functions for easier access to crs in GeoTiffCrs ([#142](https://github.com/gadomski/las-rs/pull/142))
- *(deps)* bump actions/create-github-app-token from 2.2.1 to 3.0.0 ([#139](https://github.com/gadomski/las-rs/pull/139))

## [0.9.10](https://github.com/gadomski/las-rs/compare/v0.9.9...v0.9.10) - 2026-02-04

### Fixed

- re-add writer drop implementation ([#137](https://github.com/gadomski/las-rs/pull/137))

## [0.9.9](https://github.com/gadomski/las-rs/compare/v0.9.8...v0.9.9) - 2026-01-26

### Added

- add ability to write laz in parallel ([#133](https://github.com/gadomski/las-rs/pull/133))

### Other

- fix vars instead of secrets
- use orca bro for deploys ([#134](https://github.com/gadomski/las-rs/pull/134))

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
