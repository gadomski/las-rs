//! Counts the number of points in a las file.

extern crate las;

use las::Reader;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Must provide a path to a las file");
    let mut reader = Reader::from_path(path).expect("Unable to open reader");
    let npoints = reader.points().collect::<Vec<Result<_, _>>>().len();
    println!("Number of points: {}", npoints);
}
