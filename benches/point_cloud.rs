//! Micro-benchmark comparing the per-point `Point` API against the byte-slab
//! `PointCloud` API on a fixed batch of decompressed points.
//!
//! Point to a LAZ file by setting the `LAS_BENCH_LAZ` environment variable; if
//! it is unset, the benchmark runs against `tests/data/autzen.laz`. For a
//! meaningful signal, use a file with several million points.

use criterion::{criterion_group, criterion_main, Criterion};
use las::{PointCloud, Reader};
use std::{env, hint::black_box};

const BATCH: u64 = 10_000_000;

fn bench_path() -> String {
    env::var("LAS_BENCH_LAZ").unwrap_or_else(|_| "tests/data/autzen.laz".to_string())
}

fn bench(c: &mut Criterion) {
    let path = bench_path();
    let mut group = c.benchmark_group("point_cloud_vs_points");
    group.sample_size(10);

    group.bench_function("read_points_into (Vec<Point>)", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let mut points = Vec::with_capacity(BATCH as usize);
            let n = reader.read_points_into(BATCH, &mut points).unwrap();
            black_box(n);
            black_box(points.len());
        });
    });

    group.bench_function("read_into_cloud (PointCloud)", |b| {
        b.iter(|| {
            let mut reader = Reader::from_path(&path).unwrap();
            let mut cloud = PointCloud::new(
                *reader.header().point_format(),
                *reader.header().transforms(),
            );
            let n = reader.read_into_cloud(&mut cloud, BATCH).unwrap();
            black_box(n);
            black_box(cloud.len());
        });
    });

    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
