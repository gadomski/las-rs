//! Las points.

use super::{Error, Result};

/// A las point.
///
/// As we do for the `Header` we encasulate different point formats by using `Option<T>`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// The x value of the point.
    ///
    /// This value is assumed to not include a scaling and offset -- in other words, this is the
    /// "true" value, not the value stored in the lasfile.
    pub x: f64,
    /// The y value of the point.
    ///
    /// This value is assumed to not include a scaling and offset -- in other words, this is the
    /// "true" value, not the value stored in the lasfile.
    pub y: f64,
    /// The z value of the point.
    ///
    /// This value is assumed to not include a scaling and offset -- in other words, this is the
    /// "true" value, not the value stored in the lasfile.
    pub z: f64,
    /// The intensity of the point.
    pub intensity: u16,
    /// The return number of the point for its pulse.
    ///
    /// TODO these aren't actually u8s.
    pub return_number: u8,
    /// The number of returns in total for the pulse that produced this point.
    pub number_of_returns: u8,
    /// The `ScanDirection` of the point.
    pub scan_direction: ScanDirection,
    /// True if this point is on the edge of a flight line.
    pub edge_of_flight_line: bool,
    /// The `Classification` of this point.
    pub classification: Classification,
    /// The scan angle range -- basically, the integer value of the scan angle.
    pub scan_angle_rank: i8,
    /// Custom data that the user can provide.
    pub user_data: u8,
    /// The "file" from which this point originated.
    pub point_source_id: u16,
    /// The gps time of this point.
    pub gps_time: Option<f64>,
    /// The red channel for this point.
    pub red: Option<u16>,
    /// The green channel for this point.
    pub green: Option<u16>,
    /// The blue channel for this point.
    pub blue: Option<u16>,
}

/// The scan direction of the mirror when this point was collected.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanDirection {
    /// The scan was moving backward, whatever that means.
    Backward = 0,
    /// The scan was moving forward, whatever that means.
    Forward = 1,
}

impl Default for ScanDirection {
    fn default() -> ScanDirection {
        ScanDirection::Forward
    }
}

impl ScanDirection {
    /// Translates a u8 into a scan direction.
    ///
    /// Returns an error if the u8 is not zero or one.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::ScanDirection;
    /// assert_eq!(ScanDirection::Backward, ScanDirection::from_u8(0);
    /// assert_eq!(ScanDirection::Forward, ScanDirection::from_u8(1);
    /// ```
    pub fn from_u8(n: u8) -> Result<ScanDirection> {
        match n {
            0 => Ok(ScanDirection::Backward),
            1 => Ok(ScanDirection::Forward),
            _ => Err(Error::InvalidScanDirection(n)),
        }
    }
}

/// The classification of a point.
///
/// We allow mising docs on the classes because the names say it all.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Classification {
    CreatedNeverClassified = 0,
    Unclassified = 1,
    Ground = 2,
    LowVegetation = 3,
    MediumVegetation = 4,
    HighVegetation = 5,
    Building = 6,
    LowPoint = 7,
    ModelKeyPoint = 8,
    Water = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Overlap = 12,
    Reserved,
}

impl Default for Classification {
    fn default() -> Classification {
        Classification::CreatedNeverClassified
    }
}

impl From<u8> for Classification {
    fn from(n: u8) -> Self {
        match n {
            0 => Classification::CreatedNeverClassified,
            1 => Classification::Unclassified,
            2 => Classification::Ground,
            3 => Classification::LowVegetation,
            4 => Classification::MediumVegetation,
            5 => Classification::HighVegetation,
            6 => Classification::Building,
            7 => Classification::LowPoint,
            8 => Classification::ModelKeyPoint,
            9 => Classification::Water,
            10 => Classification::Reserved10,
            11 => Classification::Reserved11,
            12 => Classification::Overlap,
            _ => Classification::Reserved,
       }
    }
}
