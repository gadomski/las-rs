//! Read ASPRS las files.

#![deny(missing_copy_implementations, missing_debug_implementations, missing_docs, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_extern_crates,
        unused_import_braces, unused_qualifications)]

extern crate byteorder;
extern crate rustc_serialize;

use std::result;

#[macro_use] pub mod macros;
pub mod header;
pub mod io;
pub mod point;
pub mod reader;
pub mod util;
pub mod vlr;

pub use header::Header;
pub use point::Point;
pub use reader::Reader;
pub use vlr::Vlr;

#[derive(Debug)]
pub enum Error {
    ByteorderError(byteorder::Error),
    CharacterAfterNullByte,
    /// A scan direction is either a zero or a one, nothing else.
    InvalidScanDirection(u8),
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
