//! Counts the number of points in a las file.

extern crate las;

use las::Reader;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Must provide a path to a las file");
    let reader = Reader::from_path(path).expect("Unable to open reader");
    let npoints = reader.header().number_of_points();
    println!("Number of points: {npoints}");
}
