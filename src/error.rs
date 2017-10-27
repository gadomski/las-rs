use {Transform, Version, header};
use point::Format;
use std::io;
use std::str;

quick_error! {
    /// Crate-specific error enum.
    #[derive(Debug)]
    pub enum Error {
        /// The value can't have the inverse transform applied.
        InverseTransform(n: f64, transform: Transform) {
            description("cannot apply inverse transform")
            display("the transform {} cannot be inversely applied to {}", transform, n)
        }
        /// The writer is closed.
        ClosedWriter {
            description("the writer is closed")
        }
        /// The las data is laszip compressed.
        Laszip {
            description("the las data is laszip compressed, and laszip compression is not supported by this build")
        }
        /// A wrapper around `las::header::Error`.
        Header(err: header::Error) {
            from()
            cause(err)
            description("las header error")
            display("header error: {}", err)
        }
        /// This string is not ASCII.
        NotAscii(s: String) {
            description("the string is not an ascii string")
            display("this string is not ascii: {}", s)
        }
        /// These bytes are not zero-filled.
        NotZeroFilled(bytes: Vec<u8>) {
            description("the bytes are not zero filled")
            display("the bytes are not zero filled: {:?}", bytes)
        }
        /// An invalid classification number.
        InvalidClassification(n: u8) {
            description("invalid classification")
            display("invalid classification: {}", n)
        }
        /// This is an invalid format.
        ///
        /// It has a combination of options that can't exist.
        InvalidFormat(format: Format) {
            description("invalid format")
            display("invalid format: {:?}", format)
        }
        /// This is an invalid format number.
        InvalidFormatNumber(n: u8) {
            description("invalid format number")
            display("invalid format number: {:?}", n)
        }
        /// This is not a valid return number.
        InvalidReturnNumber(n: u8, version: Option<Version>) {
            description("invalid return number")
            display("invalid return number: {} (for version: {:?})", n, version)
        }
        /// This is not a valid scanner channel
        InvalidScannerChannel(n: u8) {
            description("invalid scanner channel")
            display("the scanner channel is invalid: {}", n)
        }
        /// Wrapper around `std::io::Error`.
        Io(err: io::Error) {
            from()
            cause(err)
            description(err.description())
            display("io error: {}", err)
        }
        /// This string is too long for the target slice.
        StringTooLong(s: String, len: usize) {
            description("the string is too long for the target slice")
            display("string is too long for a slice of length {}: {}", len, s)
        }
        /// Wrapper around `std::str::Utf8Error`.
        Utf8(err: str::Utf8Error) {
            from()
            cause(err)
            description(err.description())
            display("utf8 error: {}", err)
        }
        /// This version does not support the feature.
        VersionDoesNotSupport(version: Version, s: String) {
            description("las version does not support the provided feature")
            display("las version {} does not support feature: {}", version, s)
        }
        /// The data in the VLR are too long for LAS.
        VlrDataTooLong(n: usize) {
            description("the data in the vlr are too long for las")
            display("vlr data of length {} are too long", n)
        }
    }
}
