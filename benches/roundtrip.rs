extern crate criterion;
extern crate las;

use criterion::{criterion_group, criterion_main, Criterion};
use las::{Point, Reader, Writer};
use std::hint::black_box;

fn roundtrip(npoints: usize) {
    let mut writer = Writer::default();
    for _ in 0..npoints {
        writer.write_point(Point::default()).unwrap();
    }
    let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
    for point in reader.points() {
        let _ = point.unwrap();
    }
}

fn bench(criterion: &mut Criterion) {
    for npoints in 0..4 {
        criterion.bench_function(&format!("roundtrip {npoints} points"), |b| {
            b.iter(|| roundtrip(black_box(npoints)))
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
