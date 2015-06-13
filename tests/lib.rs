extern crate las;

use std::io::Cursor;

#[test]
fn roundtrip() {
    let reader = las::Reader::open("data/1.2_0.las").unwrap();
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = las::Writer::new(&mut cursor).unwrap();
    writer.write_from_reader(reader).unwrap();
    las::Reader::new(cursor).unwrap();
}
