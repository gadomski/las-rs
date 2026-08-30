use crate::{point::Format, Transform, Version};
use thiserror::Error;

/// Crate-specific error enum.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// The VLR does not identify an Extra Bytes record.
    #[error("the VLR is not an Extra Bytes VLR")]
    NotExtraBytesVlr,

    /// The writer is closed.
    #[error("the writer is closed")]
    ClosedWriter,

    /// The header size, as computed, is too large.
    #[error("the header is too large ({0} bytes) to convert to a raw header")]
    HeaderTooLarge(usize),

    /// The seek index used was too large
    #[error("Seek Index reached the end: {0}")]
    SeekIndexOutOfBounds(u64),

    /// An invalid classification number.
    #[error("invalid classification: {0}")]
    InvalidClassification(u8),

    /// If a Entry is referencing a Page in the Copc Hierarchy but the page is not present
    #[cfg(feature = "laz")]
    #[error("Entry referenced a page that is not present")]
    ReferencedPageMissingFromEvlr(crate::copc::Entry),

    /// The COPC info VLR was not found.
    #[cfg(feature = "laz")]
    #[error("copc info vlr not found")]
    CopcInfoVlrNotFound,

    /// The COPC hierarchy EVLR was not found.
    #[cfg(feature = "laz")]
    #[error("copc hierarchy evlr not found")]
    CopcHierarchyEvlrNotFound,

    /// The requested COPC resolution is invalid.
    #[cfg(feature = "laz")]
    #[error("invalid COPC resolution: {0}")]
    InvalidResolution(f64),

    /// The byte buffer length is not a multiple of the point record length.
    #[error("byte buffer length {len} is not a multiple of point record length {record_len}")]
    InvalidByteBufferLength {
        /// The buffer length.
        len: usize,
        /// The expected record length.
        record_len: usize,
    },

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

    /// The GeoTiff-CRS (E)VLR(s) could not be read. Either the ascii- or double-vlrs are missing
    #[error("Unable to read the ascii or double GeoTiff (E)VLR(s)")]
    UnreadableGeoTiffCrs,

    /// The Geotiff key entry is undefined
    #[error("Invalid location key encountered for GeotiffKeyEntry with id {0}")]
    UndefinedDataForGeoTiffKey(u16),

    /// Invalid GeoTIFF header values
    #[error("Invalid GeoTIFF header: expected key_directory_version={expected_version}, key_revision={expected_revision}, minor_revision={expected_minor}, but got key_directory_version={actual_version}, key_revision={actual_revision}, minor_revision={actual_minor}")]
    InvalidGeoTiffHeader {
        /// Expected key directory version
        expected_version: u16,
        /// Actual key directory version
        actual_version: u16,
        /// Expected key revision
        expected_revision: u16,
        /// Actual key revision
        actual_revision: u16,
        /// Expected minor revision
        expected_minor: u16,
        /// Actual minor revision
        actual_minor: u16,
    },

    /// More than one Extra Bytes VLR is present.
    #[error("found {0} Extra Bytes VLRs; at most one is allowed")]
    MultipleExtraBytesVlrs(usize),

    /// The VLR payload is not a sequence of 192-byte descriptors.
    #[error("Extra Bytes VLR payload length {0} is not a multiple of 192")]
    InvalidExtraBytesVlrLength(usize),

    /// A descriptor uses an unassigned data type whose width is unknown.
    #[error("Extra Bytes descriptor uses reserved data type {0}")]
    ReservedExtraBytesDataType(u8),

    /// A descriptor uses a data type that typed writing does not support.
    #[error("cannot write Extra Bytes descriptor data type {0}")]
    UnsupportedExtraBytesDataType(u8),

    /// The descriptors require more bytes than the point format contains.
    #[error(
        "Extra Bytes VLR describes {0} bytes, but the point format contains only {1} extra bytes"
    )]
    ExtraBytesMismatch(usize, usize),

    /// A byte slab does not contain enough Extra Bytes for a descriptor.
    #[error("PointData contains {1} extra bytes per point, but the field requires at least {0}")]
    PointDataExtraBytesMismatch(usize, usize),

    /// An owned point does not contain enough Extra Bytes for a descriptor.
    #[error("point contains {1} extra bytes, but the field requires at least {0}")]
    PointExtraBytesMismatch(usize, usize),

    /// No descriptor has the requested name.
    #[error("Extra Bytes field '{0}' does not exist")]
    ExtraBytesFieldNotFound(String),

    /// The requested descriptor is raw bytes or a deprecated array, not a scalar.
    #[error("Extra Bytes field '{0}' is not a scalar numeric field")]
    NonNumericExtraBytesField(String),

    /// A value cannot be represented by an Extra Bytes field's storage type.
    #[error("value cannot be encoded for Extra Bytes field '{0}'")]
    InvalidExtraBytesValue(String),

    /// An Extra Bytes field has no no-data value to encode.
    #[error("Extra Bytes field '{0}' does not define a no-data value")]
    ExtraBytesNoDataNotDefined(String),

    /// A raw Extra Bytes value has the wrong width.
    #[error("Extra Bytes field '{name}' requires {expected} bytes, but received {actual}")]
    ExtraBytesFieldLengthMismatch {
        /// The field name.
        name: String,
        /// The descriptor's byte width.
        expected: usize,
        /// The supplied value's byte width.
        actual: usize,
    },

    /// A typed Extra Bytes column has the wrong number of values.
    #[error("Extra Bytes column requires {expected} values, but received {actual}")]
    ExtraBytesColumnLengthMismatch {
        /// The number of points in the destination.
        expected: usize,
        /// The supplied number of values.
        actual: usize,
    },

    /// More than one descriptor uses the same Extra Bytes field name.
    #[error("duplicate Extra Bytes field name '{0}'")]
    DuplicateExtraBytesField(String),
}
