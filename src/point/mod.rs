//! Attributed three-dimensional points.
//!
//! Points are simple structures with public attributes, some optional.
//!
//! ```
//! use las::Point;
//! let point = Point::default();
//! assert_eq!(0., point.x);
//! assert_eq!(None, point.color);
//! ```
//!
//! Point coordinates (x, y, and z) are stored as f64, and are the final coordinates after the
//! scale and offset from the header are applied.

mod classification;
mod format;
mod scan_direction;

pub use self::classification::Classification;
pub use self::format::Format;
pub use self::scan_direction::ScanDirection;

use {Color, Result, Transform, Vector};
use raw;
use raw::point::Waveform;

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
            display("invalid format: {}", format)
        }
        /// This is an invalid format number.
        FormatNumber(n: u8) {
            description("invalid format number")
            display("invalid format number: {}", n)
        }
        /// Overlap points are handled by an attribute on `las::Point`, not by a classification.
        OverlapClassification {
            description("Overlap points are handled by the `is_overlap` member of `las::Point`, not by classifications")
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

/// A three dimensional point.
#[derive(Clone, Debug, Default, PartialEq)]
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
    pub return_number: u8,

    /// The total number of returns for a given pulse.
    pub number_of_returns: u8,

    /// The direction at which the scanner mirror was traveling at the time of the output pulse.
    pub scan_direction: ScanDirection,

    /// True if the point is at the end of a scan.
    pub is_edge_of_flight_line: bool,

    /// The ASPRS classification for this point.
    pub classification: Classification,

    /// This point was created by a technique other than LiDAR collection.
    pub is_synthetic: bool,

    /// The point should be considered a model key-point.
    pub is_key_point: bool,

    /// The point should be considered withheld (i.e. it's deleted).
    pub is_withheld: bool,

    /// Is this an overlap point?
    pub is_overlap: bool,

    /// The channel of the scanner, used only in multi-channel systems.
    pub scanner_channel: u8,

    /// The angle of the output of the laser pulse.
    ///
    /// This is supposed to include the roll of the aircraft, if applicable. Zero degrees is nadir,
    /// -90° is to the left.
    pub scan_angle: f32,

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

    /// This point's waveform information.
    pub waveform: Option<Waveform>,

    /// This point's near infrared value.
    pub nir: Option<u16>,

    /// This point's extra bytes.
    ///
    /// These can have structure and meaning, but for now they don't.
    pub extra_bytes: Vec<u8>,
}

impl Point {
    /// Creates a point from a raw point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Point;
    /// use las::raw;
    /// let raw_point = raw::Point::default();
    /// let point = Point::new(raw_point, &Default::default());
    /// ```
    pub fn new(mut raw_point: raw::Point, transforms: &Vector<Transform>) -> Point {
        let is_overlap = raw_point.flags.is_overlap();
        raw_point.flags.clear_overlap_class();

        Point {
            x: transforms.x.direct(raw_point.x),
            y: transforms.y.direct(raw_point.y),
            z: transforms.z.direct(raw_point.z),
            intensity: raw_point.intensity,
            return_number: raw_point.flags.return_number(),
            number_of_returns: raw_point.flags.number_of_returns(),
            scan_direction: raw_point.flags.scan_direction(),
            is_edge_of_flight_line: raw_point.flags.is_edge_of_flight_line(),
            classification: raw_point.flags.to_classification().expect(
                "Overlap classification should have been cleared",
            ),
            is_synthetic: raw_point.flags.is_synthetic(),
            is_key_point: raw_point.flags.is_key_point(),
            is_withheld: raw_point.flags.is_withheld(),
            is_overlap: is_overlap,
            scan_angle: raw_point.scan_angle.into(),
            scanner_channel: raw_point.flags.scanner_channel(),
            user_data: raw_point.user_data,
            point_source_id: raw_point.point_source_id,
            gps_time: raw_point.gps_time,
            color: raw_point.color,
            waveform: raw_point.waveform,
            nir: raw_point.nir,
            extra_bytes: raw_point.extra_bytes,
        }
    }
    /// Creates a raw las point from this point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Point;
    /// let point = Point::default();
    /// let raw_point = point.into_raw(&Default::default()).unwrap();
    /// ```
    pub fn into_raw(self, transforms: &Vector<Transform>) -> Result<raw::Point> {
        Ok(raw::Point {
            x: transforms.x.inverse(self.x)?,
            y: transforms.y.inverse(self.y)?,
            z: transforms.z.inverse(self.z)?,
            intensity: self.intensity,
            flags: self.flags()?,
            scan_angle: self.scan_angle.into(),
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            gps_time: self.gps_time,
            color: self.color,
            waveform: self.waveform,
            nir: self.nir,
            extra_bytes: self.extra_bytes,
        })
    }

    /// Creates the flags bytes for use in a raw point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Point;
    /// let point = Point { return_number: 1, ..Default::default() };
    /// assert_eq!((1, 0, 0), point.flags().unwrap().into());
    /// ```
    pub fn flags(&self) -> Result<raw::point::Flags> {
        if self.return_number > 15 {
            Err(Error::ReturnNumber(self.return_number, None).into())
        } else if self.number_of_returns > 15 {
            Err(Error::ReturnNumber(self.number_of_returns, None).into())
        } else if self.scanner_channel > 3 {
            Err(Error::ScannerChannel(self.scanner_channel).into())
        } else {
            let a = (self.number_of_returns << 4) + self.return_number;
            let mut b = self.scanner_channel << 4;
            if self.is_synthetic {
                b += 1;
            }
            if self.is_key_point {
                b += 2;
            }
            if self.is_withheld {
                b += 4;
            }
            if self.is_overlap {
                b += 8;
            }
            if self.scan_direction == ScanDirection::LeftToRight {
                b += 64;
            }
            if self.is_edge_of_flight_line {
                b += 128;
            }
            Ok(raw::point::Flags::ThreeByte(
                a,
                b,
                self.classification.into(),
            ))
        }
    }

    /// Returns true if this point matches the point format.
    ///
    /// "Matches" means that the set of optional attributes is exactly the same.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Format;
    /// use las::Point;
    ///
    /// let mut format = Format::new(0).unwrap();
    /// let mut point = Point::default();
    /// assert!(point.matches(&format));
    ///
    /// format.has_gps_time = true;
    /// assert!(!point.matches(&format));
    ///
    /// point.gps_time = Some(42.);
    /// assert!(point.matches(&format));
    /// ```
    pub fn matches(&self, format: &Format) -> bool {
        self.gps_time.is_some() == format.has_gps_time &&
            self.color.is_some() == format.has_color &&
            self.waveform.is_some() == format.has_waveform &&
            self.nir.is_some() == format.has_nir &&
            self.extra_bytes.len() == format.extra_bytes as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_invalid_return_number() {
        assert!(
            Point {
                return_number: 16,
                ..Default::default()
            }.flags()
                .is_err()
        );
    }

    #[test]
    fn flags_invalid_number_of_returns() {
        assert!(
            Point {
                number_of_returns: 16,
                ..Default::default()
            }.flags()
                .is_err()
        );
    }

    #[test]
    fn flags_invalid_scanner_channel() {
        assert!(
            Point {
                scanner_channel: 4,
                ..Default::default()
            }.flags()
                .is_err()
        );
    }

    #[test]
    fn overlap() {
        use raw::point::Flags;

        let raw_point = raw::Point {
            flags: Flags::TwoByte(0, 12),
            ..Default::default()
        };
        let point = Point::new(raw_point, &Default::default());
        assert_eq!(Classification::Unclassified, point.classification);
        assert!(point.is_overlap);

    }
}
