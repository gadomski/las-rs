#[test]
#[cfg(feature = "laz")]
fn read_invalid_file() {
    // https://github.com/gadomski/las-rs/pull/61
    let mut reader = las::Reader::from_path("tests/data/32-1-472-150-76.laz").unwrap();
    let pd = reader.read_points(1).unwrap();
    let _ = pd.points().next().unwrap().unwrap();
}
