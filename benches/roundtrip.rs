#![feature(test)]

extern crate las;
extern crate test;

use las::{Point, Reader, Writer};
use test::Bencher;

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

#[bench]
fn roundtrip_0(bencher: &mut Bencher) {
    bencher.iter(|| roundtrip(0));
}

#[bench]
fn roundtrip_1(bencher: &mut Bencher) {
    bencher.iter(|| roundtrip(1));
}

#[bench]
fn roundtrip_100(bencher: &mut Bencher) {
    bencher.iter(|| roundtrip(100));
}

#[bench]
fn roundtrip_10000(bencher: &mut Bencher) {
    bencher.iter(|| roundtrip(10000));
}
