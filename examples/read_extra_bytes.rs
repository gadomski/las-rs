extern crate las;

use las::point::extra_bytes::ExtraBytesReader;
use las::{Read, Reader};

fn main() {
    let path = std::env::args()
        .skip(1)
        .next()
        .expect("Must provide a path to a las file");

    let mut reader = Reader::from_path(&path).expect("Unable to open reader");
    let header = reader.header();

    // Read the ExtraBytes field definition from the Header
    let ebr = ExtraBytesReader::new(&header);

    if ebr.has_extra_bytes() {
        println!(
            "LAS file contains ExtraByte attributes, named: {:?}",
            ebr.names()
        );
        println!("Description of fields: {:?}", ebr.descriptions());

        let mut ct = 0;
        for p in reader.points() {
            let pt = p.expect("Unable to read point");

            // we can read all extra attributes
            println!("{:?}", ebr.all_values(&pt));

            // or we can use a vec of strings for which we expect to find extra attributes
            // (fields with these names should be present in the las header as extra bytes structs)
            vec![
                String::from("range"),
                String::from("phi"),
                String::from("time"),
            ]
            .iter()
            .for_each(|name| {
                println!("{} -> {}", name, ebr.value_for_named_field(&name, &pt));
            });

            // stop reading after 10 points
            ct += 1;
            if ct >= 10 {
                break;
            }
        }
    }
}
