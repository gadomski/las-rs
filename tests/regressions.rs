use las::{Builder, Reader, Writer};
use tempfile::NamedTempFile;

#[test]
fn issue_136() {
    let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    let mut points: Vec<las::Point> =
        Vec::with_capacity(reader.header().number_of_points() as usize);
    let _ = reader.read_all_points_into(&mut points);

    let mut builder = Builder::from((1, 4));
    builder.point_format = las::point::Format::new(1).unwrap();
    let header = builder.into_header().unwrap();

    let tempfile = NamedTempFile::new().unwrap();
    let file_name = tempfile.path().to_str().unwrap().to_string();
    {
        let mut writer = Writer::from_path(&file_name, header).unwrap();
        writer.write_points(&points).unwrap();
    }

    let reader = Reader::from_path(file_name).unwrap();
    assert_eq!(reader.header().number_of_points(), 106);
}
