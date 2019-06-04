extern crate las;

use las::Reader;

#[test]
fn detect_laszip() {
    if cfg!(feature = "lazperf-compression") {
        assert!(Reader::from_path("tests/data/autzen.laz").is_ok());
    } else {
        assert!(Reader::from_path("tests/data/autzen.laz").is_err());
    }
}


#[cfg(feature = "lazperf-compression")]
mod lazperf_compression_test {
    use std::io::{Cursor, SeekFrom, Seek};


    /// Read file, write it compressed, read the compressed data written
    /// compare that points are the same
    fn test_compression_does_not_corrupt(path: &str) {
        let mut reader = las::Reader::from_path(path).expect("Cannot open reader");
        let points: Vec<las::Point> = reader.points().map(|r| r.unwrap()).collect();

        let mut header_builder = las::Builder::from(reader.header().version());
        header_builder.point_format = reader.header().point_format().clone();
        header_builder.point_format.is_compressed = true;

        let header = header_builder.into_header().unwrap();
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut writer = las::Writer::new(cursor, header).unwrap();

        for point in &points {
            writer.write(point.clone()).unwrap();
        }
        writer.close().unwrap();
        let mut cursor = writer.into_inner().unwrap();

        cursor.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = las::Reader::new(cursor).unwrap();
        let points_2: Vec<las::Point> = reader.points().map(|r| r.unwrap()).collect();

        assert_eq!(points, points_2);
    }

    #[test]
    fn test_autzen_laz() {
        test_compression_does_not_corrupt("tests/data/autzen.laz");
    }

    #[test]
    fn test_autzen_las() {
        test_compression_does_not_corrupt("tests/data/autzen.las");
    }


    #[test]
    fn test_extra_bytes_laz() {
        test_compression_does_not_corrupt("tests/data/extrabytes.laz");
    }
}
