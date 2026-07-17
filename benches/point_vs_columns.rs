//! Compares the two ways of reading a whole file: decoding every record
//! into an owned [`las::Point`] via [`las::PointData::points`] versus
//! walking the byte columns directly via [`las::PointData::x`] and
//! friends.
//!
//! Both variants materialize a [`las::PointData`] via
//! [`las::Reader::read_all`] first, then sum the x/y/z components of
//! every point — the compiler can't elide the work because the result is
//! `black_box`'d. The ratio between them approximates the cost of
//! `raw::Point::read_from` + full [`las::Point`] materialization per
//! record versus three cheap column scans that only touch the bytes
//! they need.
//!
//! The default input is `tests/data/autzen.las`, which is small — numbers
//! are noisy and mostly useful as a smoke test. For a meaningful signal,
//! point to a larger file with `LAS_BENCH_FILE`:
//!
//! ```sh
//! # Uncompressed LAS:
//! LAS_BENCH_FILE=/path/to/large.las cargo bench --bench point_vs_columns
//! # LAZ requires the `laz` feature (or `laz-parallel` for throughput):
//! LAS_BENCH_FILE=/path/to/large.laz cargo bench --bench point_vs_columns --features laz-parallel
//! ```
//!
//! Public LAS/LAZ tiles are available from USGS 3DEP
//! (<https://www.usgs.gov/3d-elevation-program>) and OpenTopography
//! (<https://opentopography.org>).

use criterion::{criterion_group, criterion_main, Criterion};
use las::Reader;
use std::{env, hint::black_box};

fn bench_path() -> String {
    env::var("LAS_BENCH_FILE").unwrap_or_else(|_| "tests/data/autzen.las".to_string())
}

fn bench(c: &mut Criterion) {
    let path = bench_path();
    let mut group = c.benchmark_group("point_vs_columns");
    group.sample_size(10);

    // Per-`Point` path: decode every record in the slab as an owned
    // `Point`, paying the full decode + `Point::new` cost per record.
    group.bench_function("read_all().points() (per-Point)", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let pd = reader.read_all().unwrap();
            let mut sum = 0.0f64;
            let mut n = 0u64;
            for p in pd.points() {
                let p = p.unwrap();
                sum += p.x + p.y + p.z;
                n += 1;
            }
            black_box(n);
            black_box(sum);
        });
    });

    // Byte-slab column path: sweep three columns independently — no
    // `Point` struct ever constructed.
    group.bench_function("read_all() + columns", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let points = reader.read_all().unwrap();
            let sum: f64 =
                points.x().sum::<f64>() + points.y().sum::<f64>() + points.z().sum::<f64>();
            black_box(points.len());
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
