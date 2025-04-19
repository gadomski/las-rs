use crate::{point::Format, Transform, Version};
use thiserror::Error;

/// Crate-specific error enum.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// The writer is closed.
    #[error("the writer is closed")]
    ClosedWriter,

    /// The laszip vlr was not found, the points cannot be decompressed.
    #[cfg(feature = "laz")]
    #[error("copcinfo vlr not found")]
    CopcInfoVlrNotFound,
    /// The laszip vlr was not found, the points cannot be decompressed.
    #[cfg(feature = "laz")]
    #[error("copchierarchy vlr not found")]
    CopcHierarchyVlrNotFound,

    /// The header size, as computed, is too large.
    #[error("the header is too large ({0} bytes) to convert to a raw header")]
    HeaderTooLarge(usize),

    /// The seek index used was too large
    #[error("Seek Index reached the end: {0}")]
    SeekIndexOutOfBounds(u64),

    /// An invalid classification number.
    #[error("invalid classification: {0}")]
    InvalidClassification(u8),

    /// The file signature is not LASF.
    #[error("the file signature is not 'LASF': {0:?}")]
    InvalidFileSignature([u8; 4]),

    /// The value can't have the inverse transform applied.
    #[error("the transform {transform} cannot be inversely applied to {n}")]
    InvalidInverseTransform {
        /// The float.
        n: f64,

        /// The transform that can't be applied
        transform: Transform,
    },

    /// This is an invalid point format.
    ///
    /// It has a combination of options that can't exist.
    #[error("invalid point format: {0}")]
    InvalidPointFormat(Format),

    /// This is an invalid format number.
    #[error("invalid format number: {0}")]
    InvalidPointFormatNumber(u8),

    /// This is not a valid scanner channel
    #[error("invalid scanner channel: {0}")]
    InvalidScannerChannel(u8),

    /// [std::io::Error]
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The las data is laszip compressed.
    #[error("the las data is laszip compressed, but the las crate is not built with laz support")]
    LaszipNotEnabled,

    /// [laz::LasZipError]
    #[cfg(feature = "laz")]
    #[error(transparent)]
    LasZipError(#[from] laz::LasZipError),

    /// The laszip vlr was not found, the points cannot be decompressed.
    #[cfg(feature = "laz")]
    #[error("laszip vlr not found")]
    LasZipVlrNotFound,

    /// This string is not ASCII.
    #[error("this string is not ascii: {0}")]
    NotAscii(String),

    /// These bytes are not zero-filled.
    #[error("the bytes are not zero-filled: {0:?}")]
    NotZeroFilled(Vec<u8>),

    /// The offset to the start of the evlrs is too small.
    #[error("offset to the start of the evlrs is too small: {0}")]
    OffsetToEvlrsTooSmall(u64),

    /// The offset to point data is too large.
    #[error("the offset to the point data is too large: {0}")]
    OffsetToPointDataTooLarge(usize),

    /// The offset to the point data was too small.
    #[error("offset to the point data is too small: {0}")]
    OffsetToPointDataTooSmall(u32),

    /// Overlap points are handled by an attribute on [Point](crate::Point), not by a classification.
    #[error("overlap points are handled by the `is_overlap` member of `las::Point`, not by classifications")]
    OverlapClassification,

    /// The attributes of the point format and point do not match.
    #[error("the attributes of the point format ({0}) do not match the point")]
    PointAttributesDoNotMatch(Format),

    /// The point data record length is too small for the format.
    #[error("the point data record length {len} is too small for format {format}")]
    PointDataRecordLengthTooLarge {
        /// The point format.
        format: Format,

        /// The length of the point data record.
        len: u16,
    },

    /// Point padding is only allowed when evlrs are present.
    #[error("point padding is only allowed when evlrs are present")]
    PointPaddingNotAllowed,

    /// This is not a valid return number.
    #[error("invalid return number {return_number} for version {version:?}")]
    ReturnNumber {
        /// The return number.
        return_number: u8,

        /// The version that doesn't support this return number.
        version: Option<Version>,
    },

    /// This string is too long for the target slice.
    #[error("string is too long for a slice of length {len}: {string}")]
    StringTooLong {
        /// The string.
        string: String,

        /// The target length.
        len: usize,
    },

    /// Too many extended variable length records.
    #[error("too many extended variable length records: {0}")]
    TooManyEvlrs(usize),

    /// Too many points for this version.
    #[error("too many points for version {version}: {n}")]
    TooManyPoints {
        /// The number of points.
        n: u64,

        /// The LAS version
        version: Version,
    },

    /// Too many variable length records.
    #[error("too many variable length records: {0}")]
    TooManyVlrs(usize),

    /// [std::num::TryFromIntError]
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),

    /// Feature is not supported by version.
    #[error("feature {feature} is not supported by version {version}")]
    UnsupportedFeature {
        /// The LAS version.
        version: Version,

        /// The feature that is not supported
        feature: &'static str,
    },

    /// The point format is not supported by version.
    #[error("version {version} does not support format {format}")]
    UnsupportedFormat {
        /// The LAS version.
        version: Version,

        /// The unsupported point format.
        format: Format,
    },

    /// Returned when a Function needs the arguments to be in a specific range
    #[error("Direction not in intended range (0<=direction<=7). Was {0}")]
    InvalidDirection(
        ///The Argument that does not meet the requrement
        i32,
    ),

    /// [std::str::Utf8Error]
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    /// Wkt is required for this point format.
    #[error("wkt is required for this point format: {0}")]
    WktRequired(Format),

    /// The vlr data is too long.
    #[error("the vlr is too long: {0}")]
    VlrTooLong(usize),
}
