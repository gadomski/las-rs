extern crate criterion;
extern crate las;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use las::{Point, Read, Reader, Write, Writer};

fn roundtrip(npoints: usize) {
    let mut writer = Writer::default();
    for _ in 0..npoints {
        writer.write(Point::default()).unwrap();
    }
    let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
    for point in reader.points() {
        let _ = point.unwrap();
    }
}

fn bench(criterion: &mut Criterion) {
    for npoints in 0..4 {
        criterion.bench_function(&format!("roundtrip {} points", npoints), |b| {
            b.iter(|| roundtrip(black_box(npoints)))
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);
