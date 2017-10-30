//! Massage and work the auzten file to make sure we can deal with real data.

extern crate las;

macro_rules! autzen {
    ($name:ident, $major:expr, $minor:expr) => {
        mod $name {
            use std::io::Cursor;
            use las::{Version, Reader, Writer};

            #[test]
            fn read_write() {
                let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
                let mut header = reader.header.clone();
                header.version = Version::new($major, $minor);
                let mut writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
                for point in reader.points() {
                    writer.write(point.unwrap()).unwrap();
                }
                writer.close().unwrap();
            }
        }
    }
}

autzen!(las_1_0, 1, 0);
autzen!(las_1_1, 1, 1);
autzen!(las_1_2, 1, 2);
autzen!(las_1_3, 1, 3);
autzen!(las_1_4, 1, 4);
