//! Our errors.

use std::error;
use std::io;
use std::fmt;

use byteorder;

use header;

/// Crate-specific errors.
#[derive(Debug)]
pub enum Error {
    /// Wraps `byteorder::Error`.
    Byteorder(byteorder::Error),
    /// Invalid classification value.
    InvalidClassification(u8),
    /// Point number of returns was out of bounds.
    InvalidNumberOfReturns(u8),
    /// Unrecognized point data format.
    InvalidPointFormat(u8),
    /// Point return number was out of allowed bounds.
    InvalidReturnNumber(u8),
    /// Wraps `std::io::Error`.
    Io(io::Error),
    /// Tried to write a point to a format it doesn't support.
    PointFormat(header::PointFormat, String),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Byteorder(ref err) => err.description(),
            Error::InvalidClassification(_) => "invalid classification",
            Error::InvalidNumberOfReturns(_) => "invalid number of returns",
            Error::InvalidPointFormat(_) => "invalid point data format",
            Error::InvalidReturnNumber(_) => "invalid return number",
            Error::Io(ref err) => err.description(),
            Error::PointFormat(_, _) => "point format mismatch",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Byteorder(ref err) => Some(err),
            Error::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Byteorder(ref err) => write!(f, "Byteorder error: {}", err),
            Error::InvalidClassification(n) => write!(f, "Invalid classification: {}", n),
            Error::InvalidNumberOfReturns(n) => write!(f, "Invalid number of returns: {}", n),
            Error::InvalidPointFormat(n) => write!(f, "Invalid point data format: {}", n),
            Error::InvalidReturnNumber(n) => write!(f, "Invalid return number: {}", n),
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::PointFormat(format, ref field) => write!(f, "Point format mismatch for format '{}' and field '{}'", format, field),
        }
    }
}

impl From<byteorder::Error> for Error {
    fn from(err: byteorder::Error) -> Error {
        Error::Byteorder(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

