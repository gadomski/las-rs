use las::{ExtraBytesColumn, ExtraBytesVlr, Reader};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Must provide a path to a las file");

    let mut reader = Reader::from_path(&path).unwrap();

    // Parse and validate the Extra Bytes descriptors once.
    let extra_bytes = ExtraBytesVlr::new(reader.header()).unwrap().unwrap();
    dbg!(&extra_bytes);

    if !extra_bytes.has_extra_bytes() {
        println!("LAS file has no Extra Bytes");
        return;
    }

    // Read the extra bytes column-wise
    println!("\nThe 10 first values per extra field read columnwise:");
    let points = reader.read_all().unwrap();
    for descriptor in extra_bytes.descriptors() {
        println!(
            "{}: {} ({:?})",
            descriptor.name(),
            descriptor.description(),
            descriptor.data_type()
        );
        if descriptor.data_type().is_scalar() {
            match points.extra_column(descriptor).unwrap().unwrap() {
                ExtraBytesColumn::Unsigned(values) => {
                    println!("unsigned values: {:?}", values.take(10).collect::<Vec<_>>());
                }
                ExtraBytesColumn::Signed(values) => {
                    println!("signed values: {:?}", values.take(10).collect::<Vec<_>>());
                }
                ExtraBytesColumn::Float(values) => {
                    println!("float values: {:?}", values.take(10).collect::<Vec<_>>());
                }
            }
        } else {
            let values: Vec<_> = points
                .extra_column_raw(descriptor)
                .unwrap()
                .unwrap()
                .take(10)
                .collect();
            println!("raw values: {values:?}");
        }
    }
    // or read them point-wise
    println!("\nThe same 10 first values per extra field read pointwise:");
    for point in points.points().map(|p| p.unwrap()).take(10) {
        for descriptor in extra_bytes.descriptors() {
            if descriptor.data_type().is_scalar() {
                let value = point.extra_field(descriptor).unwrap();
                println!("{}: {value:?}", descriptor.name());
            } else {
                let value = point.extra_field_raw(descriptor).unwrap();
                println!("(raw) {}: {value:?}", descriptor.name());
            }
        }
    }

    if extra_bytes.undocumented_bytes_len() > 0 {
        println!(
            "{} trailing bytes per point are not described by the VLR",
            extra_bytes.undocumented_bytes_len()
        );
    }
}
