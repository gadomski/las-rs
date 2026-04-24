//! Massage and work the auzten file to make sure we can deal with real data.

macro_rules! autzen {
    ($name:ident, $major:expr_2021, $minor:expr_2021) => {
        mod $name {
            use las::{Builder, Reader, Version, Writer};
            use std::io::Cursor;

            #[test]
            fn read_write() {
                let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
                let mut builder = Builder::from(reader.header().clone());
                builder.version = Version::new($major, $minor);
                let mut writer =
                    Writer::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap();
                for point in reader.read_all().unwrap().points() {
                    writer.write_point(point.unwrap()).unwrap();
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
    use las::Reader;
    let mut reader = Reader::from_path(path).unwrap();
    assert_eq!(reader.read_points(1).unwrap().len(), 1);
    reader.seek(0).unwrap();
    assert_eq!(reader.read_points(1).unwrap().len(), 1);
}

fn test_seek_to_last_point_works_on(path: &str) {
    use las::Reader;
    let mut reader = Reader::from_path(path).unwrap();
    assert_eq!(reader.read_points(1).unwrap().len(), 1);
    reader.seek(reader.header().number_of_points() - 1).unwrap();
    assert_eq!(reader.read_points(1).unwrap().len(), 1);
}

fn test_seek_past_last_point_works_on(path: &str) {
    use las::Reader;
    let mut reader = Reader::from_path(path).unwrap();
    assert_eq!(reader.read_points(1).unwrap().len(), 1);
    reader.seek(reader.header().number_of_points()).unwrap();
    assert!(reader.read_points(1).unwrap().is_empty());
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

#[cfg(feature = "laz")]
#[test]
fn test_seek_past_last_point_works_on_copc() {
    test_seek_past_last_point_works_on("tests/data/autzen.copc.laz");
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

// TODO: This test hangs in LasZipDecompressor::seek
// #[cfg(feature = "laz")]
// #[test]
// fn test_seek_to_last_point_works_on_copc() {
//     test_seek_to_last_point_works_on("tests/data/autzen.copc.laz");
// }

#[test]
fn test_seek_0_works_on_las() {
    test_seek_0_works_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_seek_0_works_on_laz() {
    test_seek_0_works_on("tests/data/autzen.laz");
}

// TODO: This test hangs in LasZipDecompressor::seek
// #[cfg(feature = "laz")]
// #[test]
// fn test_seek_0_works_on_copc() {
//     test_seek_0_works_on("tests/data/autzen.copc.laz");
// }

fn test_read_points_on(path: &str) {
    use las::{Point, Reader};

    let ground_truth_points = {
        let mut ground_truth_reader = Reader::from_path(path).unwrap();
        ground_truth_reader
            .read_all()
            .unwrap()
            .points()
            .collect::<las::Result<Vec<Point>>>()
            .unwrap()
    };

    let mut reader = Reader::from_path(path).unwrap();
    let n = 7;
    let mut all_points = Vec::with_capacity(reader.header().number_of_points() as usize);
    loop {
        let pd = reader.read_points(n).unwrap();
        if pd.is_empty() {
            break;
        }
        for p in pd.points() {
            all_points.push(p.unwrap());
        }
    }

    assert_eq!(all_points, ground_truth_points);
}

fn test_fill_points_on(path: &str) {
    use las::{Point, PointData, Reader};

    let ground_truth_points = {
        let mut ground_truth_reader = Reader::from_path(path).unwrap();
        ground_truth_reader
            .read_all()
            .unwrap()
            .points()
            .collect::<las::Result<Vec<Point>>>()
            .unwrap()
    };

    let mut reader = Reader::from_path(path).unwrap();
    let n = 7;
    let mut all_points = Vec::with_capacity(reader.header().number_of_points() as usize);
    let mut buffer = PointData::new(
        *reader.header().point_format(),
        *reader.header().transforms(),
    );
    while reader.fill_points(n, &mut buffer).unwrap() != 0 {
        for p in buffer.points() {
            all_points.push(p.unwrap());
        }
    }

    assert_eq!(all_points, ground_truth_points);
}

#[test]
fn test_las_read_n() {
    test_read_points_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_laz_read_n() {
    test_read_points_on("tests/data/autzen.laz");
}

#[cfg(feature = "laz")]
#[test]
fn test_copc_read_n() {
    test_read_points_on("tests/data/autzen.copc.laz");
}

#[test]
fn test_las_fill_points() {
    test_fill_points_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_laz_fill_points() {
    test_fill_points_on("tests/data/autzen.laz");
}

#[cfg(feature = "laz")]
#[test]
fn test_copc_fill_points() {
    test_fill_points_on("tests/data/autzen.copc.laz");
}

fn test_read_all_on(path: &str) {
    use las::{Point, Reader};

    let ground_truth_points = {
        let mut ground_truth_reader = Reader::from_path(path).unwrap();
        ground_truth_reader
            .read_all()
            .unwrap()
            .points()
            .collect::<las::Result<Vec<Point>>>()
            .unwrap()
    };

    let mut reader = Reader::from_path(path).unwrap();
    let pd = reader.read_all().unwrap();
    let all_points: Vec<Point> = pd
        .points()
        .collect::<las::Result<Vec<Point>>>()
        .unwrap();
    assert_eq!(all_points, ground_truth_points);
}

#[test]
fn test_las_read_all() {
    test_read_all_on("tests/data/autzen.las");
}

#[cfg(feature = "laz")]
#[test]
fn test_laz_read_all() {
    test_read_all_on("tests/data/autzen.laz");
}

#[cfg(feature = "laz")]
#[test]
fn test_copc_read_all() {
    test_read_all_on("tests/data/autzen.copc.laz");
}

#[cfg(feature = "laz-parallel")]
#[test]
fn test_seek_to_zero() {
    use las::Reader;
    use std::fs::File;

    // https://github.com/gadomski/las-rs/issues/125
    let file = File::open("tests/data/autzen.copc.laz").unwrap();
    let mut reader = Reader::new(file).unwrap();
    for _ in reader.read_points(1_000).unwrap().points() {}
    reader.seek(0).unwrap();
}
