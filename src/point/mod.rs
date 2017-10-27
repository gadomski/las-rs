//! Three-dimensional points with additional attributes.

mod classification;
mod format;
mod point;
mod scan_direction;

pub use point::classification::Classification;
pub use point::format::Format;
pub use point::point::Point;
pub use point::scan_direction::ScanDirection;

quick_error! {
    /// Point-specific errors
    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        /// An invalid classification number.
        Classification(n: u8) {
            description("invalid classification")
            display("invalid classification: {}", n)
        }
        /// This is an invalid format.
        ///
        /// It has a combination of options that can't exist.
        Format(format: Format) {
            description("invalid format")
            display("invalid format: {:?}", format)
        }
        /// This is an invalid format number.
        FormatNumber(n: u8) {
            description("invalid format number")
            display("invalid format number: {:?}", n)
        }
        /// This is not a valid return number.
        ReturnNumber(n: u8, version: Option<::Version>) {
            description("invalid return number")
            display("invalid return number: {} (for version: {:?})", n, version)
        }
        /// This is not a valid scanner channel
        ScannerChannel(n: u8) {
            description("invalid scanner channel")
            display("the scanner channel is invalid: {}", n)
        }
    }
}
