use std::io;
use std::str;

use point::{Format, Point};

/// Crate-specific error enum.
#[derive(Debug)]
pub enum Error {
    /// The `Writer` is closed and cannot be written to.
    ClosedWriter,
    /// Wrapper around `std::str::Utf8Error`.
    Utf8(str::Utf8Error),
    /// The file signature was not "LASF".
    InvalidFileSignature(String),
    /// The point data record length is less than the point format demands.
    InvalidPointDataRecordLength(Format, u16),
    /// Wrapper around `std::io::Error`.
    Io(io::Error),
    /// The point format requires color, but the point did not have color set.
    MissingColor(Format, Point),
    /// The point format requires gps time, but the point did not have gps time set.
    MissingGpsTime(Format, Point),
    /// This string is not ASCII, and it was suppoed to be.
    NotAscii(String),
    /// The buffer was not filled with nuls after the last ASCII character.
    NotNulFilled(Vec<u8>),
    /// The reserved field is not zero.
    ReservedIsNotZero,
    /// The point format is not supported by this library.
    ///
    /// It might be valid, but we just can't handle it.
    UnsupportedPointFormat(Format),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(err: str::Utf8Error) -> Error {
        Error::Utf8(err)
    }
}
