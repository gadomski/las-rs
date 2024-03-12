//! Massage and work the auzten file to make sure we can deal with real data.

extern crate las;

macro_rules! autzen {
    ($name:ident, $major:expr, $minor:expr) => {
        mod $name {
            use las::{Builder, Read, Reader, Version, Write, Writer};
            use std::io::Cursor;

            #[test]
            fn read_write() {
                let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
                let mut builder = Builder::from(reader.header().clone());
                builder.version = Version::new($major, $minor);
                let mut writer =
                    Writer::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap();
                for point in reader.points() {
                    writer.write(point.unwrap()).unwrap();
                }
                writer.close().unwrap();
            }
        }
    };
}

autzen!(las_1_0, 1, 0);
autzen!(las_1_1, 1, 1);
autzen!(las_1_2, 1, 2);
autzen!(las_1_3, 1, 3);
autzen!(las_1_4, 1, 4);

fn test_seek_0_works_on(path: &str) {
    use las::{Read, Reader};
    let mut reader = Reader::from_path(path).unwrap();
    let _p1 = reader.read().unwrap().unwrap();
    reader.seek(0).unwrap();
    let _p3 = reader.read().unwrap().unwrap();
}

fn test_seek_to_last_point_works_on(path: &str) {
    use las::{Read, Reader};
    let mut reader = Reader::from_path(path).unwrap();
    let _p1 = reader.read().unwrap().unwrap();
    reader.seek(reader.header().number_of_points() - 1).unwrap();
    let res = reader.read();
    assert!(res.is_some());
    assert!(res.unwrap().is_ok());
}

fn test_seek_past_last_point_works_on(path: &str) {
    use las::{Read, Reader};
    let mut reader = Reader::from_path(path).unwrap();
    let _p1 = reader.read().unwrap().unwrap();
    reader.seek(reader.header().number_of_points()).unwrap();
    let res = reader.read();
    assert!(res.is_none());
}

#[test]
fn test_seek_past_last_point_works_on_las() {
    test_seek_past_last_point_works_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_seek_past_last_point_works_on_laz() {
    test_seek_past_last_point_works_on("tests/data/autzen.laz");
}

#[test]
fn test_seek_to_last_point_works_on_las() {
    test_seek_to_last_point_works_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_seek_to_last_point_works_on_laz() {
    test_seek_to_last_point_works_on("tests/data/autzen.laz");
}

#[test]
fn test_seek_0_works_on_las() {
    test_seek_0_works_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_seek_0_works_on_laz() {
    test_seek_0_works_on("tests/data/autzen.laz");
}
