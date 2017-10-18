use point::{Format, RawPoint};
use std::error;
use std::fmt;
use std::io;
use std::str;

/// Crate-specific error enum.
#[derive(Debug)]
pub enum Error {
    /// The writer is closed.
    ClosedWriter,
    /// The header size is too small.
    HeaderSizeTooSmall(u16),
    /// The las data is laszip compressed.
    Laszip,
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
    /// The value can't be represnted as an i32.
    Overflow(f64),
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
            Error::ClosedWriter => write!(f, "The writer is closed"),
            Error::HeaderSizeTooSmall(n) => {
                write!(f, "The header was {} bytes, which is too small", n)
            }
            Error::Laszip => write!(f, "The las data is laszip compressed"),
            Error::OffsetToDataTooSmall(n) => {
                write!(f, "The offset to data was {} bytes, which is too small", n)
            }
            Error::MissingColor(format, _) => {
                write!(
                    f,
                    "Point was missing color information, required by {}",
                    format
                )
            }
            Error::MissingGpsTime(format, _) => {
                write!(
                    f,
                    "Point was missing GPS time information, required by {}",
                    format
                )
            }
            Error::NotAscii(ref s) => write!(f, "The string {} is not ASCII", s),
            Error::NotZeroFilled(ref v) => write!(f, "The vector {:?} was not zero filled", v),
            Error::InvalidNumberOfReturns(n) => write!(f, "{} is not a valid number of returns", n),
            Error::InvalidReturnNumber(n) => write!(f, "{} is not a valid return number", n),
            Error::Overflow(n) => write!(f, "{} cannot be represented as a i32", n),
            Error::VersionDoesNotSupport(version, ref s) => {
                write!(
                    f,
                    "Version {}.{} does not support {}",
                    version.0,
                    version.1,
                    s
                )
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

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ClosedWriter => "the writer is closed",
            Error::HeaderSizeTooSmall(_) => "header size is too small",
            Error::Laszip => "this library does not support laszip (yet)",
            Error::OffsetToDataTooSmall(_) => "offset to point data is too small",
            Error::MissingColor(_, _) => "color data is missing",
            Error::MissingGpsTime(_, _) => "gps time data is missing",
            Error::NotAscii(_) => "the string is not ascii",
            Error::NotZeroFilled(_) => "the string is not zero filled",
            Error::InvalidNumberOfReturns(_) => "invalid number of returns",
            Error::InvalidReturnNumber(_) => "invalid return number",
            Error::Io(ref err) => err.description(),
            Error::Overflow(_) => "number cannot be represented as an i32",
            Error::StringTooLong(_, _) => "string is too long",
            Error::Utf8(ref err) => err.description(),
            Error::VersionDoesNotSupport(_, _) => "version does not support feature",
            Error::VlrDataTooLong(_) => "vlr data is too long",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref err) => Some(err),
            Error::Utf8(ref err) => Some(err),
            _ => None,
        }
    }
}
