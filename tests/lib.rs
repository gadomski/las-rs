extern crate las;

use std::io::Cursor;

#[test]
fn roundtrip() {
    let mut reader = las::Reader::open("data/1.2_0.las").unwrap();
    let header = reader.header().clone();
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = las::Writer::new(&mut cursor).unwrap();
    writer.write_from_reader(reader).unwrap();
    reader = las::Reader::new(cursor).unwrap();
    assert_eq!(&header, reader.header());
}
