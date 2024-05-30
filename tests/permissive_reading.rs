#[test]
#[cfg(feature = "laz")]
fn read_invalid_file() {
    // https://github.com/gadomski/las-rs/pull/61
    use las::Read;

    let mut reader = las::Reader::from_path("tests/data/32-1-472-150-76.laz").unwrap();
    let _ = reader.points().next().unwrap().unwrap();
}
