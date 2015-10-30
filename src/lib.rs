//! Read ASPRS las files.

#![deny(missing_copy_implementations, missing_debug_implementations, missing_docs, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_extern_crates,
        unused_import_braces, unused_qualifications)]

extern crate byteorder;
extern crate rustc_serialize;

use std::error::Error;
use std::fmt;
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

/// Crate-specific errors.
#[derive(Debug)]
pub enum LasError {
    /// Wrapper around a byteorder::Error.
    Byteorder(byteorder::Error),
    /// A reader found a non-null character after a null byte when reading a las string.
    CharacterAfterNullByte,
    /// A scan direction is either a zero or a one, nothing else.
    InvalidScanDirection(u8),
    /// Wrapper around an io::Error.
    Io(std::io::Error),
    /// Some sort of error occurred while reading.
    Read(String),
}

impl fmt::Display for LasError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LasError::Byteorder(ref err) => write!(f, "Byteorder error: {}", err),
            LasError::CharacterAfterNullByte => write!(f, "Found a character after a null byte"),
            LasError::InvalidScanDirection(number) => write!(f, "Invalid scan direction: {}", number),
            LasError::Io(ref err) => write!(f, "IO error: {}", err),
            LasError::Read(ref string) => write!(f, "Read error: {}", string),
        }
    }
}

impl Error for LasError {
    fn description(&self) -> &str {
        match *self {
            LasError::Byteorder(ref err) => err.description(),
            LasError::CharacterAfterNullByte => "character after a null byte",
            LasError::InvalidScanDirection(_) => "invalid scan direction",
            LasError::Io(ref err) => err.description(),
            LasError::Read(_) => "read error",
        }

    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            LasError::Byteorder(ref err) => Some(err),
            LasError::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for LasError {
    fn from(err: std::io::Error) -> LasError {
        LasError::Io(err)
    }
}

impl From<byteorder::Error> for LasError {
    fn from(err: byteorder::Error) -> LasError {
        LasError::Byteorder(err)
    }
}

/// Crate-specific result type.
pub type Result<T> = result::Result<T, LasError>;
