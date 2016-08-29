//! Wrappers around other crate's errors, and our own custom errors.

use std::error;
use std::io;
use std::fmt;

use byteorder;

use header;

/// Errors.
#[derive(Debug)]
pub enum Error {
    /// Wraps `byteorder::Error`.
    Byteorder(byteorder::Error),
    /// The provided `u8` does not correspond to a classification value.
    InvalidClassification(u8),
    /// The las format specifies an upper bound on the number of returns for a given pulse. This
    /// error is returned when the provided `u8` exceeds that maximum bound.
    InvalidNumberOfReturns(u8),
    /// The provided `u8` cannot be mapped onto a (suppored) point data format.
    ///
    /// Note that the `u8` might be allowed under a version of the las standard, but if this
    /// library doesn't support that point format, this error will be returned.
    InvalidPointFormat(u8),
    /// Similarly to the number of returns, a point's return number has a maximum bound. This error
    /// is returned if the `u8` exceeds that bound.
    InvalidReturnNumber(u8),
    /// Wraps `std::io::Error`.
    Io(io::Error),
    /// Returned if a point cannot be translated to the required point format, due to missing
    /// dimensions.
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
            Error::PointFormat(format, ref field) => {
                write!(f,
                       "Point format mismatch for format '{}' and field '{}'",
                       format,
                       field)
            }
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
