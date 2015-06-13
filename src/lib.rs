//! Read and write ASPRS las files.

extern crate byteorder;
extern crate rustc_serialize;

use std::result;

mod header;
mod point;
mod reader;
mod writer;

pub use header::Header;
pub use point::Point;
pub use reader::Reader;
pub use writer::Writer;

#[derive(Debug)]
enum Error {
    ByteorderError(byteorder::Error),
    CharacterAfterNullByte,
    IoError(std::io::Error),
    ReadError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<byteorder::Error> for Error {
    fn from(err: byteorder::Error) -> Error {
        Error::ByteorderError(err)
    }
}

pub type Result<T> = result::Result<T, Error>;
