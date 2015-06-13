//! Read and write ASPRS las files.

use std::result;

mod point;
mod reader;
mod writer;

pub use point::Point;
pub use reader::Reader;
pub use writer::Writer;

#[derive(Debug)]
enum Error {

}

pub type Result<T> = result::Result<T, Error>;
