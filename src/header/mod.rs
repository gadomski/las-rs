//! Metadata describing the layout and interpretation of the points.

mod header;
mod raw;

pub use header::header::Header;
pub use header::raw::RawHeader;

const HEADER_SIZE: u16 = 227;

/// The meaning of GPS time in the point records.
#[derive(Clone, Copy, Debug)]
pub enum GpsTimeType {
    /// GPS Week Time (the same as previous versions of LAS).
    Week,
    /// Standard GPS Time minu 1e9.
    Standard,
}
