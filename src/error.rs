use std::fmt;
use std::io;
use std::str;

use point::{Format, RawPoint};

/// Crate-specific error enum.
#[derive(Debug)]
pub enum Error {
    /// The header size is too small.
    HeaderSizeTooSmall(u16),
    /// The offset to data is too small.
    OffsetToDataTooSmall(u32),
    /// This format requires color, but it is missing.
    MissingColor(Format, RawPoint),
    /// This format requires GPS time, but it is missing.
    MissingGpsTime(Format, RawPoint),
    /// This string is not ASCII.
    NotAscii(String),
    /// These bytes are not zero-filled.
    NotZeroFilled(Vec<u8>),
    /// This is not a valid number of returns.
    InvalidNumberOfReturns(u8),
    /// This is not a valid return number.
    InvalidReturnNumber(u8),
    /// Wrapper around `std::io::Error`.
    Io(io::Error),
    /// This string is too long for the target slice.
    StringTooLong(String, usize),
    /// Wrapper around `std::str::Utf8Error`.
    Utf8(str::Utf8Error),
    /// This version does not support the feature.
    VersionDoesNotSupport((u8, u8), String),
    /// The data in the VLR are too long for LAS.
    VlrDataTooLong(usize),
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::HeaderSizeTooSmall(n) => {
                write!(f, "The header was {} bytes, which is too small", n)
            }
            Error::OffsetToDataTooSmall(n) => {
                write!(f, "The offset to data was {} bytes, which is too small", n)
            }
            Error::MissingColor(format, _) => {
                write!(f,
                       "Point was missing color information, required by {}",
                       format)
            }
            Error::MissingGpsTime(format, _) => {
                write!(f,
                       "Point was missing GPS time information, required by {}",
                       format)
            }
            Error::NotAscii(ref s) => write!(f, "The string {} is not ASCII", s),
            Error::NotZeroFilled(ref v) => write!(f, "The vector {:?} was not zero filled", v),
            Error::InvalidNumberOfReturns(n) => write!(f, "{} is not a valid number of returns", n),
            Error::InvalidReturnNumber(n) => write!(f, "{} is not a valid return number", n),
            Error::VersionDoesNotSupport(version, ref s) => {
                write!(f,
                       "Version {}.{} does not support {}",
                       version.0,
                       version.1,
                       s)
            }
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::StringTooLong(ref s, n) => {
                write!(f, "String is too long for a field of {} bytes: {}", n, s)
            }
            Error::Utf8(ref err) => write!(f, "Utf8 error: {}", err),
            Error::VlrDataTooLong(n) => write!(f, "VLR data too long: {} bytes", n),
        }
    }
}
