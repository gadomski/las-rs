use std::io;
use std::str;
use thiserror::Error;
use crate::{header, point, reader, vlr, writer, Transform, Version};

/// Crate-specific error enum.
#[derive(Error, Debug)]
pub enum Error {
    /// Feature is not supported by version.
    #[error("feature {feature} is not supported by version {version}")]
    #[allow(missing_docs)]
    Feature {
        version: Version,
        feature: &'static str,
    },

    /// A wrapper around `las::header::Error`.
    #[error(transparent)]
    Header(#[from] header::Error),

    /// The value can't have the inverse transform applied.
    #[error("the transform {transform} cannot be inversely applied to {n}")]
    #[allow(missing_docs)]
    InverseTransform { n: f64, transform: Transform },

    /// Wrapper around `std::io::Error`.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// The las data is laszip compressed.
    #[error(
        "the las data is laszip compressed, but laszip compression is not supported by this build"
    )]
    Laszip,

    /// This string is not ASCII.
    #[error("this string is not ascii: {0}")]
    NotAscii(String),

    /// These bytes are not zero-filled.
    #[error("the bytes are not zero-filled: {0:?}")]
    NotZeroFilled(Vec<u8>),

    /// Wrapper around `las::point::Error`.
    #[error(transparent)]
    Point(#[from] point::Error),

    /// Wrapper around `las::reader::Error`.
    #[error(transparent)]
    Reader(#[from] reader::Error),

    /// This string is too long for the target slice.
    #[error("string is too long for a slice of length {len}: {string}")]
    #[allow(missing_docs)]
    StringTooLong { string: String, len: usize },

    /// Wrapper around `std::str::Utf8Error`.
    #[error(transparent)]
    Utf8(#[from] str::Utf8Error),

    /// Wrapper around `las::writer::Error`.
    #[error(transparent)]
    Writer(#[from] writer::Error),

    /// Wrapper around `las::vlr::Error`.
    #[error(transparent)]
    Vlr(#[from] vlr::Error),

    /// Wrapper around `laz::LasZipError`
    #[cfg(feature = "laz")]
    #[error("laszip error: {0}")]
    LasZipError(laz::LasZipError),

    /// The Laszip vlr was not found, the points cannot be decompressed
    #[cfg(feature = "laz")]
    #[error("laszip vlr not found")]
    LasZipVlrNotFound,
}

#[cfg(feature = "laz")]
impl From<laz::LasZipError> for Error {
    fn from(error: laz::LasZipError) -> Error {
        Error::LasZipError(error)
    }
}
