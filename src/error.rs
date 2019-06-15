use {Transform, Version, header, point, reader, vlr, writer};
use std::io;
use std::str;

quick_error! {
    /// Crate-specific error enum.
    #[derive(Debug)]
    pub enum Error {
        /// Feature is not supported by version.
        Feature(version: Version, feature: &'static str) {
            description("feature is not supported by version")
            display("feature {} is not supported by version {}", feature, version)
        }
        /// A wrapper around `las::header::Error`.
        Header(err: header::Error) {
            from()
            cause(err)
            description("las header error")
            display("header error: {}", err)
        }
        /// The value can't have the inverse transform applied.
        InverseTransform(n: f64, transform: Transform) {
            description("cannot apply inverse transform")
            display("the transform {} cannot be inversely applied to {}", transform, n)
        }
        /// Wrapper around `std::io::Error`.
        Io(err: io::Error) {
            from()
            cause(err)
            description(err.description())
            display("io error: {}", err)
        }
        /// The las data is laszip compressed.
        Laszip {
            description("the las data is laszip compressed, and laszip compression is not supported by this build")
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
        /// Wrapper around `las::point::Error`.
        Point(err: point::Error) {
            from()
            cause(err)
            description("point error")
            display("point error: {}", err)
        }
        /// Wrapper around `las::reader::Error`.
        Reader(err: reader::Error) {
            from()
            cause(err)
            description("reader error")
            display("reader error: {}", err)
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
        /// Wrapper around `las::writer::Error`.
        Writer(err: writer::Error) {
            from()
            cause(err)
            description("writer error")
            display("writer error: {}", err)
        }
        /// Wrapper around `las::vlr::Error`.
        Vlr(err: vlr::Error) {
            from()
            cause(err)
            description("vlr error")
            display("vlr error: {}", err)
        }

        /// Wrapper around `laz::LasZipError`
        #[cfg(feature = "laz")]
        LasZipError(err: laz::LasZipError) {
            description("Error related to LAZ")
            display("LasZipError: {}", err)
        }
    }
}

// Because quick_error! fails to propagate the #[cfg(feature...)]
// if "from()' is in the LasZipError enum variant declaration
// resulting in a error when compiling las-rs without laz
// telling that module laz is undeclared.
// We have to implement it ourselves

#[cfg(feature = "laz")]
impl From<laz::LasZipError> for Error {
    fn from(e: laz::LasZipError) -> Self {
        Error::LasZipError(e)
    }
}
