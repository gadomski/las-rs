use point::{Classification, Color, ScanDirection};

/// A point is the basic unit of information in LAS data.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// The x coordinate, as a float.
    pub x: f64,
    /// The y coordinate, as a float.
    pub y: f64,
    /// The z coordinate, as a float.
    pub z: f64,
    /// The integer representation of the pulse return magnitude.
    ///
    /// This value is optional and system specific, but should be included when available. Because
    /// there is no way to indicate the "optionalness" of the intensity value and since zero could
    /// be valid intensity, we don't wrap this in an `Option`.
    pub intensity: u16,
    /// The pulse return number for a given output pulse.
    ///
    /// Valid number are one through five for LAS 1.0 through 1.3.
    ///
    /// TODO LAS 1.4?
    pub return_number: u8,
    /// The total number of returns for a given pulse.
    pub number_of_returns: u8,
    /// The direction at which the scanner mirror was traveling at the time of the output pulse.
    pub scan_direction: ScanDirection,
    /// True if the point is at the end of a scan.
    pub edge_of_flight_line: bool,
    /// The ASPRS classification for this point.
    pub classification: Classification,
    /// This point was created by a technique other than LiDAR collection.
    pub synthetic: bool,
    /// The point should be considered a model key-point.
    pub key_point: bool,
    /// The point should be considered withheld (i.e. it's deleted).
    pub withheld: bool,
    /// The angle, rounded to the nearest integer, of the output of the laser pulse.
    ///
    /// This is supposed to include the roll of the aircraft, if applicable. Zero degrees is nadir,
    /// -90° is to the left.
    pub scan_angle_rank: i8,
    /// Used at the user's discretion.
    pub user_data: u8,
    /// The file from which this point originated.
    ///
    /// This number corresponds to a file source ID.
    pub point_source_id: u16,
    /// The time at which the point was acquired.
    pub gps_time: Option<f64>,
    /// This point's color.
    pub color: Option<Color>,
}
