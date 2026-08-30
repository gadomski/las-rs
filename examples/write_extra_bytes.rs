use las::{
    Builder, ExtraBytesDataType, ExtraBytesDescriptor, ExtraBytesVlr, Point, PointDataBuilder,
    Result, Writer,
};

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("Must provide a path for the output LAS file");

    let temperature = ExtraBytesDescriptor::new("temperature", ExtraBytesDataType::I16)?
        .with_description("Degrees Celsius")?
        .with_no_data(i16::MIN)?
        .with_scale(0.01)?;
    let quality = ExtraBytesDescriptor::new("quality", ExtraBytesDataType::U8)?;
    let extra_bytes = ExtraBytesVlr::from_descriptors([temperature, quality])?;

    let mut builder = Builder::from((1, 4));
    builder.set_extra_bytes_vlr(&extra_bytes)?;
    let mut writer = Writer::from_path(path, builder.into_header()?)?;

    let temperature = &extra_bytes.descriptors()[0];
    let quality = &extra_bytes.descriptors()[1];

    // Write the first half of the points point-wise.
    for index in 0..500 {
        let mut point = Point {
            x: f64::from(index),
            y: f64::from(index),
            ..Point::default()
        };
        extra_bytes.initialize_point(&mut point);
        point.set_extra_field(temperature, 20.0 + f64::from(index) / 10.0)?;
        point.set_extra_field(quality, (index % 255) as u8)?;
        writer.write_point(point)?;
    }

    // Write the second half column-wise. First build the point records with
    // zero-filled Extra Bytes, then populate one complete column at a time.
    let mut points = PointDataBuilder::new()
        .for_header(writer.header())
        .build_from_points((500..1_000).map(|index| {
            let mut point = Point {
                x: f64::from(index),
                y: f64::from(index),
                ..Point::default()
            };
            extra_bytes.initialize_point(&mut point);
            point
        }))?;
    points.set_extra_column(
        temperature,
        (500..1_000).map(|index| 20.0 + f64::from(index) / 10.0),
    )?;
    points.set_extra_column(quality, (500..1_000).map(|index| (index % 255) as u8))?;
    writer.write_points(&points)?;

    writer.close()
}
