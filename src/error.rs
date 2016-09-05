use std::error;
use std::fmt;
use std::io;
use std::str;

use global_encoding::GpsTime;
use point::{Format, Point};
use version::Version;

/// Crate-specific error enum.
#[derive(Debug)]
pub enum Error {
    /// The `Writer` is closed and cannot be written to.
    ClosedWriter,
    /// The `GpsTime` is not supported by this version.
    GpsTimeMismatch(Version, GpsTime),
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
    /// Wrapper around `std::str::Utf8Error`.
    Utf8(str::Utf8Error),
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

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::ClosedWriter => "the writer is closed",
            Error::GpsTimeMismatch(_, _) => "mismatch between version and gps time",
            Error::InvalidFileSignature(_) => "file signature was not LASF",
            Error::InvalidPointDataRecordLength(_, _) => "the point data record length is impossible (probably too short)",
            Error::Io(ref err) => err.description(),
            Error::MissingColor(_, _) => "color was required by the point format, but the point did not have color",
            Error::MissingGpsTime(_, _) => "gps time was required by the point format, but the point did not have gps time",
            Error::NotAscii(_) => "the string is not ascii",
            Error::NotNulFilled(_) => "the slice is not filled with nuls after the last character",
            Error::Utf8(ref err) => err.description(),
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::ClosedWriter => write!(f, "The writer is closed"),
            Error::GpsTimeMismatch(version, gps_time) => {
                write!(f, "{} does not support {}", version, gps_time)
            }
            Error::InvalidFileSignature(ref s) => {
                write!(f, "File signature must be LASF, found '{}'", s)
            }
            Error::InvalidPointDataRecordLength(format, length) => {
                write!(f,
                       "{} (with length {}) cannot support a point data record length of {}",
                       format,
                       format.record_length(),
                       length)
            }
            Error::Io(ref err) => write!(f, "IO error: {}", err),
            Error::MissingColor(format, ref point) => {
                write!(f,
                       "{} requires color, but {} doesn't have it",
                       format,
                       point)
            }
            Error::MissingGpsTime(format, ref point) => {
                write!(f,
                       "{} requires gps time, but {} doesn't have it",
                       format,
                       point)
            }
            Error::NotAscii(ref s) => write!(f, "This string is not ASCII: {}", s),
            Error::NotNulFilled(ref v) => write!(f, "This slice is not filled with nuls: {:?}", v),
            Error::Utf8(ref err) => write!(f, "UTF8 error: {}", err),
        }
    }
}
