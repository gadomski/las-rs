use point::Format;
use raw;
use std::io;
use std::str;

quick_error! {
    /// Crate-specific error enum.
    #[derive(Debug)]
    pub enum Error {
        /// The value can't be represnted as an i32.
        CannotBeI32(n: f64) {
            description("the float value cannot be an i32")
            display("the float value cannot be an i32: {}", n)
        }
        /// The writer is closed.
        ClosedWriter {
            description("the writer is closed")
        }
        /// The header size is too small.
        HeaderSizeTooSmall(header_size: u16) {
            description("the header size is too small")
            display("the header size is too small: {}", header_size)
        }
        /// The las data is laszip compressed.
        Laszip {
            description("the las data is laszip compressed, and laszip compression is not supported by this build")
        }
        /// The offset to data is too small.
        OffsetToDataTooSmall(offset: u32) {
            description("the offset to the data is too small")
            display("the offset to the data is too small: {}", offset)
        }
        /// This format requires color, but it is missing.
        MissingColor(format: Format, point: raw::Point) {
            description("the point format requires color, but it is missing")
            display("point format {} requires color, but the point doesn't have color", format)
        }
        /// This format requires GPS time, but it is missing.
        MissingGpsTime(format: Format, point: raw::Point) {
            description("the point format requires gps time, but it is missing")
            display("point format {} requires gps time, but the point doesn't have gps time", format)
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
        /// This is not a valid number of returns.
        InvalidNumberOfReturns(n: u8) {
            description("invalid number of returns")
            display("the number of returns is invalid: {}", n)
        }
        /// This is not a valid return number.
        InvalidReturnNumber(n: u8) {
            description("invalid return number")
            display("the return number is invalid: {}", n)
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
        VersionDoesNotSupport(version: (u8, u8), s: String) {
            description("las version does not support the provided feature")
            display("las version {}.{} does not support feature: {}", version.0, version.1, s)
        }
        /// The data in the VLR are too long for LAS.
        VlrDataTooLong(n: usize) {
            description("the data in the vlr are too long for las")
            display("vlr data of length {} are too long", n)
        }
    }
}
