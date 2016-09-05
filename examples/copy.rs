//! Copies a las file.

extern crate las;

use std::env;

use las::{Builder, Reader};

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let infile = args.next().expect("Too few arguments");
    let outfile = args.next().expect("Too few arguments");
    let mut reader = Reader::from_path(&infile).expect("Unable to open infile");
    let mut writer = Builder::from(&reader)
        .writer_from_path(outfile)
        .expect("Unable to create writer");
    for point in reader.iter_mut() {
        writer.write(&point.expect("Error while reading point"))
            .expect("Error while writing point");
    }
}
