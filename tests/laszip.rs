extern crate las;

use las::Reader;

#[test]
fn detect_laszip() {
    if cfg!(feature = "laz") {
        assert!(Reader::from_path("tests/data/autzen.laz").is_ok());
    } else {
        assert!(Reader::from_path("tests/data/autzen.laz").is_err());
    }
}

#[cfg(feature = "laz")]
mod laz_compression_test {
    use las::Write;
    use std::io::Cursor;

    /// Read file, write it compressed, read the compressed data written
    /// compare that points are the same
    fn test_compression_does_not_corrupt(path: &str) {
        let mut reader = las::Reader::from_path(path).expect("Cannot open reader");
        let points: Vec<las::Point> = reader.points().map(|r| r.unwrap()).collect();

        let mut header_builder = las::Builder::from(reader.header().version());
        header_builder.point_format = *reader.header().point_format();
        header_builder.point_format.is_compressed = true;

        let header = header_builder.into_header().unwrap();
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut writer = las::Writer::new(cursor, header).unwrap();

        for point in &points {
            writer.write(point.clone()).unwrap();
        }
        writer.close().unwrap();
        let cursor = writer.into_inner().unwrap();

        let mut reader = las::Reader::new(cursor).unwrap();
        let points_2: Vec<las::Point> = reader.points().map(|r| r.unwrap()).collect();

        assert_eq!(points, points_2);
    }

    #[test]
    fn test_point_format_id_is_correct() {
        let las_reader = las::Reader::from_path("tests/data/autzen.las").unwrap();
        assert_eq!(las_reader.header().point_format().to_u8().unwrap(), 1);
        let laz_reader = las::Reader::from_path("tests/data/autzen.laz").unwrap();
        assert_eq!(laz_reader.header().point_format().to_u8().unwrap(), 3);
    }

    #[test]
    fn test_autzen_las() {
        test_compression_does_not_corrupt("tests/data/autzen.las");
    }

    #[test]
    fn test_autzen_laz() {
        test_compression_does_not_corrupt("tests/data/autzen.laz");
    }

    #[test]
    fn test_extra_bytes_laz() {
        test_compression_does_not_corrupt("tests/data/extrabytes.laz");
    }
}
