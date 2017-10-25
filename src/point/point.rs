use {Color, Result, Transform, Vector, raw};
use point::{Classification, ScanDirection};

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

impl Point {
    /// Creates a point from a raw point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Point;
    /// use las::raw;
    /// let raw_point = raw::Point::default();
    /// let point = Point::new(raw_point, Default::default());
    /// ```
    pub fn new(raw_point: raw::Point, transforms: Vector<Transform>) -> Point {
        Point {
            x: transforms.x.direct(raw_point.x),
            y: transforms.y.direct(raw_point.y),
            z: transforms.z.direct(raw_point.z),
            intensity: raw_point.intensity,
            return_number: raw_point.return_number(),
            number_of_returns: raw_point.number_of_returns(),
            scan_direction: raw_point.scan_direction(),
            edge_of_flight_line: raw_point.edge_of_flight_line(),
            classification: raw_point.classification.into(),
            synthetic: raw_point.synthetic(),
            key_point: raw_point.key_point(),
            withheld: raw_point.withheld(),
            scan_angle_rank: raw_point.scan_angle_rank,
            user_data: raw_point.user_data,
            point_source_id: raw_point.point_source_id,
            gps_time: raw_point.gps_time,
            color: raw_point.color,
        }
    }
    /// Creates a raw (writable) point from this point.
    ///
    /// Raw points map pretty directly onto the attribute table provided in the LAS standard.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Point;
    /// let point = Point { ..Default::default() };
    /// let raw_point = point.to_raw(&Default::default()).unwrap();
    /// ```
    pub fn to_raw(&self, transforms: &Vector<Transform>) -> Result<::raw::Point> {
        Ok(::raw::Point {
            x: transforms.x.inverse(self.x)?,
            y: transforms.y.inverse(self.y)?,
            z: transforms.z.inverse(self.z)?,
            intensity: self.intensity,
            flags: self.flags()?,
            classification: self.classification.into(),
            scan_angle_rank: self.scan_angle_rank,
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            gps_time: self.gps_time,
            color: self.color,
        })
    }

    /// Creates the flags byte for use in a raw point.
    ///
    /// Returns an error if the return number of number of returns are out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Point;
    /// let point = Point { return_number: 1, ..Default::default() };
    /// assert_eq!(1, point.flags().unwrap());
    /// ```
    pub fn flags(&self) -> Result<u8> {
        use Error;
        let mut flags = if self.return_number <= 5 {
            self.return_number
        } else {
            return Err(Error::InvalidReturnNumber(self.return_number));
        };
        if self.number_of_returns <= 5 {
            flags += self.number_of_returns << 3
        } else {
            return Err(Error::InvalidNumberOfReturns(self.number_of_returns));
        };
        match self.scan_direction {
            ScanDirection::Positive => flags += 64,
            _ => {}
        };
        if self.edge_of_flight_line {
            flags += 128;
        }
        Ok(flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let raw_point = raw::Point {
            x: 1,
            y: 2,
            z: 3,
            ..Default::default()
        };
        let transforms = Vector {
            x: Transform {
                scale: 2.,
                offset: 1.,
            },
            y: Transform {
                scale: 3.,
                offset: 2.,
            },
            z: Transform {
                scale: 4.,
                offset: 3.,
            },
        };
        let point = Point::new(raw_point, transforms);
        assert_eq!(3., point.x);
        assert_eq!(8., point.y);
        assert_eq!(15., point.z);
    }

    #[test]
    fn to_raw() {
        let point = Point {
            x: 2.9,
            y: 8.1,
            z: 15.,
            ..Default::default()
        };
        let transforms = Vector {
            x: Transform {
                scale: 2.,
                offset: 1.,
            },
            y: Transform {
                scale: 3.,
                offset: 2.,
            },
            z: Transform {
                scale: 4.,
                offset: 3.,
            },
        };
        let point = point.to_raw(&transforms).unwrap();
        assert_eq!(1, point.x);
        assert_eq!(2, point.y);
        assert_eq!(3, point.z);
    }

    #[test]
    fn flags() {
        assert_eq!(0, Point { ..Default::default() }.flags().unwrap());
        assert_eq!(
            1,
            Point {
                return_number: 1,
                ..Default::default()
            }.flags()
                .unwrap()
        );
        assert_eq!(
            5,
            Point {
                return_number: 5,
                ..Default::default()
            }.flags()
                .unwrap()
        );
        assert!(
            Point {
                return_number: 6,
                ..Default::default()
            }.flags()
                .is_err()
        );
        assert_eq!(
            8,
            Point {
                number_of_returns: 1,
                ..Default::default()
            }.flags()
                .unwrap()
        );
        assert!(
            Point {
                number_of_returns: 6,
                ..Default::default()
            }.flags()
                .is_err()
        );
        assert_eq!(
            64,
            Point {
                scan_direction: ScanDirection::Positive,
                ..Default::default()
            }.flags()
                .unwrap()
        );
        assert_eq!(
            128,
            Point {
                edge_of_flight_line: true,
                ..Default::default()
            }.flags()
                .unwrap()
        );
    }
}
