use {Error, Point, Result, Transform, Vector};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use point::{Classification, Color, Format, ScanDirection};
use std::io::{Cursor, ErrorKind, Read, Write};

/// A raw, uninterpreted point.
#[derive(Clone, Copy, Debug, Default)]
#[allow(missing_docs)]
pub struct RawPoint {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub intensity: u16,
    pub flags: u8,
    pub classification: u8,
    pub scan_angle_rank: i8,
    pub user_data: u8,
    pub point_source_id: u16,
    pub gps_time: Option<f64>,
    pub color: Option<Color>,
}

impl RawPoint {
    /// Converts this raw point into a `Point`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { ..Default::default() };
    /// let point = raw_point.into_point(&Default::default());
    /// ```
    pub fn into_point(self, transforms: &Vector<Transform>) -> Point {
        Point {
            x: transforms.x.direct(self.x),
            y: transforms.y.direct(self.y),
            z: transforms.z.direct(self.z),
            intensity: self.intensity,
            return_number: self.return_number(),
            number_of_returns: self.number_of_returns(),
            scan_direction: self.scan_direction(),
            edge_of_flight_line: self.edge_of_flight_line(),
            classification: self.classification(),
            synthetic: self.synthetic(),
            key_point: self.key_point(),
            withheld: self.withheld(),
            scan_angle_rank: self.scan_angle_rank,
            user_data: self.user_data,
            point_source_id: self.point_source_id,
            gps_time: self.gps_time,
            color: self.color,
        }
    }

    /// Calculates the return number from the flag byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { flags: 1, ..Default::default() };
    /// assert_eq!(1, raw_point.return_number());
    /// ```
    pub fn return_number(&self) -> u8 {
        self.flags & 7
    }

    /// Calculates the number of returns from the flag byte.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { flags: 8, ..Default::default() };
    /// assert_eq!(1, raw_point.number_of_returns());
    /// ```
    pub fn number_of_returns(&self) -> u8 {
        (self.flags & 56) >> 3
    }

    /// Returns the scan direction as determined by the scan direction flag.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::{RawPoint, ScanDirection};
    /// let raw_point = RawPoint { flags: 64, ..Default::default() };
    /// assert_eq!(ScanDirection::Positive, raw_point.scan_direction());
    /// ```
    pub fn scan_direction(&self) -> ScanDirection {
        if (self.flags & 64) == 64 {
            ScanDirection::Positive
        } else {
            ScanDirection::Negative
        }
    }

    /// Returns true if the flags indicate that this point is the edge of a flight line.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { flags: 128, ..Default::default() };
    /// assert!(raw_point.edge_of_flight_line());
    /// ```
    pub fn edge_of_flight_line(&self) -> bool {
        (self.flags & 128) == 128
    }

    /// Returns the classification of this point.
    ///
    /// LAS 1.0 didn't specify the meanings behind the classifications, but all later versions did,
    /// and since 1.0 is so old we don't bother to support user-defined classifications.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::{Classification, RawPoint};
    /// let raw_point = RawPoint { classification: 2, ..Default::default() };
    /// assert_eq!(Classification::Ground, raw_point.classification());
    /// ```
    pub fn classification(&self) -> Classification {
        self.classification.into()
    }

    /// Returns true if this point is synthetic.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { classification: 32, ..Default::default() };
    /// assert!(raw_point.synthetic());
    pub fn synthetic(&self) -> bool {
        (self.classification & 32) == 32
    }

    /// Returns true if this point is a key point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { classification: 64, ..Default::default() };
    /// assert!(raw_point.key_point());
    pub fn key_point(&self) -> bool {
        (self.classification & 64) == 64
    }

    /// Returns true if this point is withheld.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::RawPoint;
    /// let raw_point = RawPoint { classification: 128, ..Default::default() };
    /// assert!(raw_point.withheld());
    pub fn withheld(&self) -> bool {
        (self.classification & 128) == 128
    }
}

impl Point {
    /// Creates a raw (writable) point from this point.
    ///
    /// Raw points map pretty directly onto the attribute table provided in the LAS standard.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Point;
    /// let point = Point { ..Default::default() };
    /// let raw_point = point.to_raw_point(&Default::default()).unwrap();
    /// ```
    pub fn to_raw_point(&self, transforms: &Vector<Transform>) -> Result<RawPoint> {
        Ok(RawPoint {
            x: transforms.x.inverse(self.x),
            y: transforms.y.inverse(self.y),
            z: transforms.z.inverse(self.z),
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

/// Reads a raw point.
pub trait ReadRawPoint {
    /// Reads a raw point.
    ///
    /// If there are exactly zero bytes left in the `Read`, then this function returns `Ok(None)`.
    ///
    /// # Examples
    ///
    /// `Read` implements `ReadRawPoint`.
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::point::ReadRawPoint;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(1994)).unwrap();
    /// let raw_point = file.read_raw_point(&1.into()).unwrap();
    /// ```
    fn read_raw_point(&mut self, format: &Format) -> Result<Option<RawPoint>>;
}

impl<R: Read> ReadRawPoint for R {
    fn read_raw_point(&mut self, format: &Format) -> Result<Option<RawPoint>> {
        let byte = match self.read_u8() {
            Ok(byte) => byte,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(err.into()),
                }
            }
        };
        let mut next_three = [0; 3];
        self.read_exact(&mut next_three)?;
        let mut cursor = Cursor::new([byte, next_three[0], next_three[1], next_three[2]]);
        let x = cursor.read_i32::<LittleEndian>()?;
        let y = self.read_i32::<LittleEndian>()?;
        let z = self.read_i32::<LittleEndian>()?;
        let intensity = self.read_u16::<LittleEndian>()?;
        let flags = self.read_u8()?;
        let classification = self.read_u8()?;
        let scan_angle_rank = self.read_i8()?;
        let user_data = self.read_u8()?;
        let point_source_id = self.read_u16::<LittleEndian>()?;
        let gps_time = if format.has_gps_time() {
            Some(self.read_f64::<LittleEndian>()?)
        } else {
            None
        };
        let color = if format.has_color() {
            let red = self.read_u16::<LittleEndian>()?;
            let green = self.read_u16::<LittleEndian>()?;
            let blue = self.read_u16::<LittleEndian>()?;
            Some(Color {
                red: red,
                green: green,
                blue: blue,
            })
        } else {
            None
        };
        Ok(Some(RawPoint {
            x: x,
            y: y,
            z: z,
            intensity: intensity,
            flags: flags,
            classification: classification,
            scan_angle_rank: scan_angle_rank,
            user_data: user_data,
            point_source_id: point_source_id,
            gps_time: gps_time,
            color: color,
        }))
    }
}

/// Writes a raw point.
pub trait WriteRawPoint {
    /// Writes a raw pont.
    ///
    /// # Examples
    ///
    /// `Write` implements `WriteRawPoint`.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::point::WriteRawPoint;
    /// let mut cursor = Cursor::new(Vec::new());
    /// cursor.write_raw_point(&Default::default(), &0.into()).unwrap();
    /// ```
    fn write_raw_point(&mut self, raw_point: &RawPoint, format: &Format) -> Result<()>;
}

impl<W: Write> WriteRawPoint for W {
    fn write_raw_point(&mut self, raw_point: &RawPoint, format: &Format) -> Result<()> {
        self.write_i32::<LittleEndian>(raw_point.x)?;
        self.write_i32::<LittleEndian>(raw_point.y)?;
        self.write_i32::<LittleEndian>(raw_point.z)?;
        self.write_u16::<LittleEndian>(raw_point.intensity)?;
        self.write_u8(raw_point.flags)?;
        self.write_u8(raw_point.classification)?;
        self.write_i8(raw_point.scan_angle_rank)?;
        self.write_u8(raw_point.user_data)?;
        self.write_u16::<LittleEndian>(raw_point.point_source_id)?;
        if format.has_gps_time() {
            if let Some(gps_time) = raw_point.gps_time {
                self.write_f64::<LittleEndian>(gps_time)?;
            } else {
                return Err(Error::MissingGpsTime(*format, *raw_point));
            }
        }
        if format.has_color() {
            if let Some(color) = raw_point.color {
                self.write_u16::<LittleEndian>(color.red)?;
                self.write_u16::<LittleEndian>(color.green)?;
                self.write_u16::<LittleEndian>(color.blue)?;
            } else {
                return Err(Error::MissingColor(*format, *raw_point));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use {Point, Transform, Vector};
    use point::{Format, ScanDirection};

    use std::io::Cursor;
    use super::*;

    #[test]
    fn return_number() {
        assert_eq!(0,
                   RawPoint { flags: 0, ..Default::default() }.return_number());
        assert_eq!(7,
                   RawPoint { flags: 7, ..Default::default() }.return_number());
        assert_eq!(0,
                   RawPoint { flags: 8, ..Default::default() }.return_number());
    }

    #[test]
    fn flags() {
        assert_eq!(0, Point { ..Default::default() }.flags().unwrap());
        assert_eq!(1,
                   Point { return_number: 1, ..Default::default() }.flags().unwrap());
        assert_eq!(5,
                   Point { return_number: 5, ..Default::default() }.flags().unwrap());
        assert!(Point { return_number: 6, ..Default::default() }.flags().is_err());
        assert_eq!(8,
                   Point { number_of_returns: 1, ..Default::default() }.flags().unwrap());
        assert!(Point { number_of_returns: 6, ..Default::default() }.flags().is_err());
        assert_eq!(64,
                   Point { scan_direction: ScanDirection::Positive, ..Default::default() }
                       .flags()
                       .unwrap());
        assert_eq!(128,
                   Point { edge_of_flight_line: true, ..Default::default() }.flags().unwrap());
    }

    #[test]
    fn number_of_returns() {
        assert_eq!(0,
                   RawPoint { flags: 0, ..Default::default() }.number_of_returns());
        assert_eq!(1,
                   RawPoint { flags: 8, ..Default::default() }.number_of_returns());
        assert_eq!(7,
                   RawPoint { flags: 56, ..Default::default() }.number_of_returns());
        assert_eq!(0,
                   RawPoint { flags: 64, ..Default::default() }.number_of_returns());
    }

    #[test]
    fn scan_direction() {
        assert_eq!(ScanDirection::Negative,
                   RawPoint { ..Default::default() }.scan_direction());
        assert_eq!(ScanDirection::Positive,
                   RawPoint { flags: 64, ..Default::default() }.scan_direction());
    }

    #[test]
    fn edge_of_flight_line() {
        assert!(!RawPoint { ..Default::default() }.edge_of_flight_line());
        assert!(RawPoint { flags: 128, ..Default::default() }.edge_of_flight_line());
    }

    #[test]
    fn write_without_gps_time() {
        let raw_point = RawPoint { ..Default::default() };
        let mut write = Cursor::new(Vec::new());
        assert!(write.write_raw_point(&raw_point, &Format::from(1)).is_err());
    }

    #[test]
    fn read_eof() {
        let mut cursor = Cursor::new(Vec::new());
        assert!(cursor.read_raw_point(&0.into()).unwrap().is_none());
    }

    #[test]
    fn read_one_byte() {
        let mut cursor = Cursor::new(vec![1]);
        assert!(cursor.read_raw_point(&0.into()).is_err());
    }

    #[test]
    fn into_point_with_transforms() {
        let raw_point = RawPoint {
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
        let point = raw_point.into_point(&transforms);
        assert_eq!(3., point.x);
        assert_eq!(8., point.y);
        assert_eq!(15., point.z);
    }

    #[test]
    fn to_raw_point_with_transforms() {
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
        let raw_point = point.to_raw_point(&transforms).unwrap();
        assert_eq!(1, raw_point.x);
        assert_eq!(2, raw_point.y);
        assert_eq!(3, raw_point.z);
    }
}
