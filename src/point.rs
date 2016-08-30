//! Three-dimensional points.
//!
//! Inside las files, x-y-z values are stored as scaled integers. We make the design decision to
//! keep scaling and offset information in the `Header` only, so that the x-y-z coordiantes of each
//! `Point` object are `f64`.
//!
//! Optional dimensions, such as color and time, are provided as `Options`. When writing points out
//! to a file, these options are checked iff the output point format requires those options.

use Result;
use error::Error;

/// A las point.
#[derive(Debug, Default, PartialEq)]
pub struct Point {
    /// The x value of the point, as a f64.
    pub x: f64,
    /// The y value of the point, as a f64.
    pub y: f64,
    /// The z value of the point, as a f64.
    pub z: f64,
    /// The pulse return magnitude.
    ///
    /// This value is system-specific.
    pub intensity: u16,
    /// The pulse return number for this point's pulse.
    pub return_number: ReturnNumber,
    /// The total number of returns in this point's pulse.
    pub number_of_returns: NumberOfReturns,
    /// The scan direction of the mirror, forwards or backwards.
    pub scan_direction: ScanDirection,
    /// Is this point at the edge of a flight line?
    pub edge_of_flight_line: bool,
    /// The classification of this point.
    ///
    /// ASPRS defines some integer-to-classification mappings.
    pub classification: Classification,
    /// Was this point created by some other means than LiDAR?
    pub synthetic: bool,
    /// Is this point a key point?
    ///
    /// If so, try not to thin it.
    pub key_point: bool,
    /// Should this point be included in processing?
    ///
    /// A.k.a. "deleted".
    pub withheld: bool,
    /// The truncated integer value of the scan angle.
    ///
    /// Negative values are left.
    pub scan_angle_rank: i8,
    /// Data used at the user's discretion.
    pub user_data: u8,
    /// The point source id.
    pub point_source_id: u16,
    /// The GPS time this point was collected.
    ///
    /// Optional, does not exist in all point formats.
    pub gps_time: Option<f64>,
    /// The red image channel, optional.
    pub red: Option<u16>,
    /// The green image channel, optional.
    pub green: Option<u16>,
    /// The blue image channel, optional.
    pub blue: Option<u16>,
    /// Any extra bytes that were included in the point record.
    ///
    /// These are legal under the standard, but they break many readers.
    pub extra_bytes: Option<Vec<u8>>,
}

impl Point {
    /// Creates a new point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Point;
    /// let point = Point::new();
    /// ```
    pub fn new() -> Point {
        Default::default()
    }
}

/// A custom wrapper to represent a point's return number.
///
/// Since the number has an upper bound, we use this wrapper to ensure those bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReturnNumber(u8);

impl ReturnNumber {
    /// Creates a return number from a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::ReturnNumber;
    /// assert!(ReturnNumber::from_u8(1).is_ok());
    /// assert!(ReturnNumber::from_u8(6).is_err());
    /// ```
    pub fn from_u8(n: u8) -> Result<ReturnNumber> {
        if n < 6 {
            Ok(ReturnNumber(n))
        } else {
            Err(Error::InvalidReturnNumber(n))
        }
    }

    /// Returns this return number as a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::ReturnNumber;
    /// assert_eq!(1, ReturnNumber::from_u8(1).unwrap().as_u8());
    /// ```
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// A custom wrapper to represent a point's number of returns.
///
/// Since the number has an upper bound, we use this wrapper to ensure those bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NumberOfReturns(u8);

impl NumberOfReturns {
    /// Creates a number of returns from a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::NumberOfReturns;
    /// assert!(NumberOfReturns::from_u8(0).is_ok());
    /// assert!(NumberOfReturns::from_u8(6).is_err());
    /// ```
    pub fn from_u8(n: u8) -> Result<NumberOfReturns> {
        if n < 6 {
            Ok(NumberOfReturns(n))
        } else {
            Err(Error::InvalidNumberOfReturns(n))
        }
    }

    /// Returns this NumberOfReturns as a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::NumberOfReturns;
    /// assert_eq!(1, NumberOfReturns::from_u8(1).unwrap().as_u8());
    /// ```
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// An enum to represent scan direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanDirection {
    /// The mirror is traveling in a negative direction, right to left.
    Backward = 0,
    /// The mirror is traveling in a positive direction, left to right.
    Forward = 1,
}

impl ScanDirection {
    /// Converts this scan direction to a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::ScanDirection;
    /// let forward = ScanDirection::Forward;
    /// assert_eq!(1, forward.as_u8());
    /// ```
    pub fn as_u8(&self) -> u8 {
        match *self {
            ScanDirection::Forward => 1,
            ScanDirection::Backward => 0,
        }
    }
}

impl Default for ScanDirection {
    fn default() -> ScanDirection {
        ScanDirection::Backward
    }
}

impl From<bool> for ScanDirection {
    fn from(b: bool) -> ScanDirection {
        if b {
            ScanDirection::Forward
        } else {
            ScanDirection::Backward
        }
    }
}

/// An enum to represent classifications.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Classification {
    /// A point is unclassified, and we haven't even tried.
    CreatedNeverClassified,
    /// This point could not be classified.
    Unclassified,
    /// Ground point.
    Ground,
    /// A low vegetation point, such as shrubbery.
    LowVegetation,
    /// Medium vegetation, like chaperelle.
    MediumVegetation,
    /// High vegetation, like forest.
    HighVegetation,
    /// A man-made building.
    Building,
    /// A noise point.
    LowPoint,
    /// A mass point, pretty synthetic.
    ModelKeyPoint,
    /// Water.
    Water,
    /// These points were culled when merging overlapping flight lines.
    Overlap,
    /// Reserved for ASPRS definition.
    ///
    /// There are several numerical values associated with `Reserved.`
    Reserved(u8),
}

impl Classification {
    /// Creates a classification from a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Classification;
    /// assert_eq!(Classification::Ground, Classification::from_u8(2).unwrap());
    /// assert!(Classification::from_u8(127).is_err());
    /// ```
    pub fn from_u8(n: u8) -> Result<Classification> {
        Ok(match n {
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
            12 => Classification::Overlap,
            10 | 11 | 13...31 => Classification::Reserved(n),
            _ => return Err(Error::InvalidClassification(n)),
        })
    }

    /// Returns this classifications u8 value.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Classification;
    /// let classification = Classification::Ground;
    /// assert_eq!(2, classification.as_u8());
    /// ```
    pub fn as_u8(&self) -> u8 {
        match *self {
            Classification::CreatedNeverClassified => 0,
            Classification::Unclassified => 1,
            Classification::Ground => 2,
            Classification::LowVegetation => 3,
            Classification::MediumVegetation => 4,
            Classification::HighVegetation => 5,
            Classification::Building => 6,
            Classification::LowPoint => 7,
            Classification::ModelKeyPoint => 8,
            Classification::Water => 9,
            Classification::Overlap => 12,
            Classification::Reserved(n) => n,
        }
    }
}

impl Default for Classification {
    fn default() -> Classification {
        Classification::CreatedNeverClassified
    }
}
