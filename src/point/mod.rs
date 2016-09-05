//! LAS points have additional information other than x, y, and z coordinates.

pub mod utils;

mod classification;
mod format;
mod number_of_returns;
mod return_number;
mod scan_direction;
mod traits;

pub use point::classification::{ASPRSClassification, Classification};
pub use point::format::Format;
pub use point::number_of_returns::NumberOfReturns;
pub use point::return_number::ReturnNumber;
pub use point::scan_direction::ScanDirection;
pub use point::traits::ReadPoint;

use std::fmt;

/// LAS point data.
#[derive(Debug, Default)]
pub struct Point {
    /// The x-coordinate of the point.
    pub x: f64,
    /// The y-coordinate of the point.
    pub y: f64,
    /// The z-coordinate of the point.
    pub z: f64,
    /// An integer representation of the pulse return magnitude.
    pub intensity: u16,
    /// The pulse return number of the given pulse.
    pub return_number: ReturnNumber,
    /// The number of returns for a given pulse.
    pub number_of_returns: NumberOfReturns,
    /// The direction at which the scanner mirror was travelling at the time of the output pulse.
    pub scan_direction: ScanDirection,
    /// True if the point is at the end of a scan.
    ///
    /// If true, this was the last point on a given scan line before it changes direction.
    pub edge_of_flight_line: bool,
    /// ASPRS standard classification.
    pub classification: Classification,
    /// The angle at which the laser point was output from the laser system, including aircraft
    /// roll.
    pub scan_angle_rank: i8,
    /// This field may be used at a user's discrescion.
    pub user_data: u8,
    /// The file from which this point originated.
    pub point_source_id: u16,
    /// The time tag value at which the point was aquired.
    pub gps_time: Option<f64>,
    /// The color associated with this point.
    pub color: Option<Color>,
    /// Any extra bytes associated with this point.
    pub extra_bytes: Vec<u8>,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Point at ({:.2},{:.2},{:.2})", self.x, self.y, self.z)
    }
}

/// The color associated with a point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Color {
    /// The red image channel associated with this point.
    pub red: u16,
    /// The green image channel associated with this point.
    pub green: u16,
    /// The blue image channel associated with this point.
    pub blue: u16,
}
