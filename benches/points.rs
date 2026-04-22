//! Micro-benchmark comparing the per-point `Point` API against the byte-slab
//! `Points` API over all points in the input file.
//!
//! The default input is `tests/data/autzen.las`, which is small — numbers are
//! noisy and mostly useful as a smoke test. For a meaningful signal, point to
//! a larger file by setting the `LAS_BENCH_FILE` environment variable:
//!
//! ```sh
//! # Uncompressed LAS:
//! LAS_BENCH_FILE=/path/to/large.las cargo bench --bench points
//! # LAZ requires the `laz` feature:
//! LAS_BENCH_FILE=/path/to/large.laz cargo bench --bench points --features laz
//! ```
//!
//! Public LAS/LAZ tiles are available from USGS 3DEP
//! (<https://www.usgs.gov/3d-elevation-program>) and OpenTopography
//! (<https://opentopography.org>).

use criterion::{criterion_group, criterion_main, Criterion};
use las::{Points, Reader};
use std::{env, hint::black_box};

fn bench_path() -> String {
    env::var("LAS_BENCH_FILE").unwrap_or_else(|_| "tests/data/autzen.las".to_string())
}

fn bench(c: &mut Criterion) {
    let path = bench_path();
    let total = Reader::from_path(&path).unwrap().header().number_of_points();
    let mut group = c.benchmark_group("points_vs_point_vec");
    group.sample_size(10);

    group.bench_function("read_points_into (Vec<Point>)", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let mut out = Vec::with_capacity(total as usize);
            let n = reader.read_points_into(total, &mut out).unwrap();
            let mut sum = 0.0f64;
            for p in &out {
                sum += p.x + p.y + p.z;
            }
            black_box(n);
            black_box(sum);
        });
    });

    group.bench_function("Points::from_reader + columns", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let points = Points::from_reader(&mut reader, total).unwrap();
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
