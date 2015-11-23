//! Reads and writes point cloud data stored in the ASPRS las file format.

#![deny(missing_copy_implementations, missing_debug_implementations, missing_docs, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_extern_crates,
        unused_import_braces, unused_qualifications)]

extern crate byteorder;

pub mod header;
pub mod file;
pub mod point;
mod io;
mod scale;
pub mod stream;
pub mod vlr;

pub use file::File;
pub use point::Point;
pub use stream::Stream;

/// Crate-specific errors.
#[derive(Debug)]
pub enum LasError {
    /// Wraps `byteorder::Error`.
    Byteorder(byteorder::Error),
    /// Invalid classification value.
    InvalidClassification(u8),
    /// Point number of returns was out of bounds.
    InvalidNumberOfReturns(u8),
    /// Unrecognized point data format.
    InvalidPointDataFormat(u8),
    /// Point return number was out of allowed bounds.
    InvalidReturnNumber(u8),
    /// Wraps `std::io::Error`.
    Io(std::io::Error),
    /// Tried to write a point to a format it doesn't support.
    PointFormat(header::PointDataFormat, String),
}

impl std::error::Error for LasError {
    fn description(&self) -> &str {
        match *self {
            LasError::Byteorder(ref err) => err.description(),
            LasError::InvalidClassification(_) => "invalid classification",
            LasError::InvalidNumberOfReturns(_) => "invalid number of returns",
            LasError::InvalidPointDataFormat(_) => "invalid point data format",
            LasError::InvalidReturnNumber(_) => "invalid return number",
            LasError::Io(ref err) => err.description(),
            LasError::PointFormat(_, _) => "point format mismatch",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            LasError::Byteorder(ref err) => Some(err),
            LasError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl std::fmt::Display for LasError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            LasError::Byteorder(ref err) => write!(f, "Byteorder error: {}", err),
            LasError::InvalidClassification(n) => write!(f, "Invalid classification: {}", n),
            LasError::InvalidNumberOfReturns(n) => write!(f, "Invalid number of returns: {}", n),
            LasError::InvalidPointDataFormat(n) => write!(f, "Invalid point data format: {}", n),
            LasError::InvalidReturnNumber(n) => write!(f, "Invalid return number: {}", n),
            LasError::Io(ref err) => write!(f, "IO error: {}", err),
            LasError::PointFormat(format, ref field) => write!(f, "Point format mismatch for format '{}' and field '{}'", format, field),
        }
    }
}

impl From<byteorder::Error> for LasError {
    fn from(err: byteorder::Error) -> LasError {
        LasError::Byteorder(err)
    }
}

impl From<std::io::Error> for LasError {
    fn from(err: std::io::Error) -> LasError {
        LasError::Io(err)
    }
}

/// Crate-specific resuls.
pub type Result<T> = std::result::Result<T, LasError>;
