//! Counts the number of points in a las file.

extern crate las;

use las::{Read, Reader};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Must provide a path to a las file");
    let mut reader = Reader::from_path(path).expect("Unable to open reader");
    let npoints = reader
        .points()
        .map(|p| p.expect("Unable to read point"))
        .count();
    println!("Number of points: {}", npoints);
}
