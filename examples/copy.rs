//! Copies a las file.

extern crate las;

use std::env;
use std::fs::File;

use las::{Builder, Reader};

fn main() {
    let mut args = env::args();
    args.next().unwrap();
    let infile = args.next().expect("Too few arguments");
    let outfile = args.next().expect("Too few arguments");
    let mut reader = Reader::from_path(&infile).expect("Unable to open infile");
    let mut writer = Builder::from_reader(&reader)
        .writer(File::create(outfile).expect("Unable to open outfile"))
        .expect("Unable to create writer");
    for point in reader.iter_mut() {
        writer.write(&point.expect("Error while reading point"))
            .expect("Error while writing point");
    }
}
