use {Color, Result};
use point::{Format, ScanDirection};
use std::io::{Cursor, ErrorKind, Read, Write};

/// A raw point.
#[derive(Clone, Copy, Debug, Default)]
#[allow(missing_docs)]
pub struct Point {
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

impl Point {
    /// Reads a raw point.
    ///
    /// If there are exactly zero bytes left in the `Read`, then this function returns `Ok(None)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::raw::Point;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(1994)).unwrap();
    /// let point = Point::read_from(file, 1.into()).unwrap();
    /// ```
    pub fn read_from<R: Read>(mut read: R, format: Format) -> Result<Option<Point>> {
        use byteorder::{LittleEndian, ReadBytesExt};
        let byte = match read.read_u8() {
            Ok(byte) => byte,
            Err(err) => {
                match err.kind() {
                    ErrorKind::UnexpectedEof => return Ok(None),
                    _ => return Err(err.into()),
                }
            }
        };
        let mut next_three = [0; 3];
        read.read_exact(&mut next_three)?;
        let mut cursor = Cursor::new([byte, next_three[0], next_three[1], next_three[2]]);
        let x = cursor.read_i32::<LittleEndian>()?;
        let y = read.read_i32::<LittleEndian>()?;
        let z = read.read_i32::<LittleEndian>()?;
        let intensity = read.read_u16::<LittleEndian>()?;
        let flags = read.read_u8()?;
        let classification = read.read_u8()?;
        let scan_angle_rank = read.read_i8()?;
        let user_data = read.read_u8()?;
        let point_source_id = read.read_u16::<LittleEndian>()?;
        let gps_time = if format.has_gps_time() {
            Some(read.read_f64::<LittleEndian>()?)
        } else {
            None
        };
        let color = if format.has_color() {
            let red = read.read_u16::<LittleEndian>()?;
            let green = read.read_u16::<LittleEndian>()?;
            let blue = read.read_u16::<LittleEndian>()?;
            Some(Color::new(red, green, blue))
        } else {
            None
        };
        // TODO read extra bytes
        Ok(Some(Point {
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

    /// Calculates the return number from the flag byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// let point = Point { flags: 1, ..Default::default() };
    /// assert_eq!(1, point.return_number());
    /// ```
    pub fn return_number(&self) -> u8 {
        self.flags & 7
    }

    /// Calculates the number of returns from the flag byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// let point = Point { flags: 8, ..Default::default() };
    /// assert_eq!(1, point.number_of_returns());
    /// ```
    pub fn number_of_returns(&self) -> u8 {
        (self.flags & 56) >> 3
    }

    /// Returns the scan direction as determined by the scan direction flag.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// use las::point::ScanDirection;
    /// let point = Point { flags: 64, ..Default::default() };
    /// assert_eq!(ScanDirection::Positive, point.scan_direction());
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
    /// use las::raw::Point;
    /// let point = Point { flags: 128, ..Default::default() };
    /// assert!(point.edge_of_flight_line());
    /// ```
    pub fn edge_of_flight_line(&self) -> bool {
        (self.flags & 128) == 128
    }

    /// Returns true if this point is synthetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// let point = Point { classification: 32, ..Default::default() };
    /// assert!(point.synthetic());
    pub fn synthetic(&self) -> bool {
        (self.classification & 32) == 32
    }

    /// Returns true if this point is a key point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// let point = Point { classification: 64, ..Default::default() };
    /// assert!(point.key_point());
    pub fn key_point(&self) -> bool {
        (self.classification & 64) == 64
    }

    /// Returns true if this point is withheld.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Point;
    /// let point = Point { classification: 128, ..Default::default() };
    /// assert!(point.withheld());
    pub fn withheld(&self) -> bool {
        (self.classification & 128) == 128
    }

    /// Writes a raw pont.
    ///
    /// # Examples
    ///
    /// `Write` implements `WriteRawPoint`.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::raw::Point;
    /// let mut cursor = Cursor::new(Vec::new());
    /// let point = Point::default();
    /// point.write_to(cursor, 0.into()).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W, format: Format) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        use Error;

        write.write_i32::<LittleEndian>(self.x)?;
        write.write_i32::<LittleEndian>(self.y)?;
        write.write_i32::<LittleEndian>(self.z)?;
        write.write_u16::<LittleEndian>(self.intensity)?;
        write.write_u8(self.flags)?;
        write.write_u8(self.classification)?;
        write.write_i8(self.scan_angle_rank)?;
        write.write_u8(self.user_data)?;
        write.write_u16::<LittleEndian>(self.point_source_id)?;
        if format.has_gps_time() {
            if let Some(gps_time) = self.gps_time {
                write.write_f64::<LittleEndian>(gps_time)?;
            } else {
                return Err(Error::MissingGpsTime(format, *self));
            }
        }
        if format.has_color() {
            if let Some(color) = self.color {
                write.write_u16::<LittleEndian>(color.red)?;
                write.write_u16::<LittleEndian>(color.green)?;
                write.write_u16::<LittleEndian>(color.blue)?;
            } else {
                return Err(Error::MissingColor(format, *self));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use point::{Format, ScanDirection};
    use std::io::Cursor;

    #[test]
    fn return_number() {
        assert_eq!(
            0,
            Point {
                flags: 0,
                ..Default::default()
            }.return_number()
        );
        assert_eq!(
            7,
            Point {
                flags: 7,
                ..Default::default()
            }.return_number()
        );
        assert_eq!(
            0,
            Point {
                flags: 8,
                ..Default::default()
            }.return_number()
        );
    }

    #[test]
    fn number_of_returns() {
        assert_eq!(
            0,
            Point {
                flags: 0,
                ..Default::default()
            }.number_of_returns()
        );
        assert_eq!(
            1,
            Point {
                flags: 8,
                ..Default::default()
            }.number_of_returns()
        );
        assert_eq!(
            7,
            Point {
                flags: 56,
                ..Default::default()
            }.number_of_returns()
        );
        assert_eq!(
            0,
            Point {
                flags: 64,
                ..Default::default()
            }.number_of_returns()
        );
    }

    #[test]
    fn scan_direction() {
        assert_eq!(
            ScanDirection::Negative,
            Point { ..Default::default() }.scan_direction()
        );
        assert_eq!(
            ScanDirection::Positive,
            Point {
                flags: 64,
                ..Default::default()
            }.scan_direction()
        );
    }

    #[test]
    fn edge_of_flight_line() {
        assert!(!Point { ..Default::default() }.edge_of_flight_line());
        assert!(
            Point {
                flags: 128,
                ..Default::default()
            }.edge_of_flight_line()
        );
    }

    #[test]
    fn write_without_gps_time() {
        let point = Point { ..Default::default() };
        let write = Cursor::new(Vec::new());
        assert!(point.write_to(write, Format::from(1)).is_err());
    }

    #[test]
    fn read_eof() {
        let cursor = Cursor::new(Vec::new());
        assert!(Point::read_from(cursor, 0.into()).unwrap().is_none());
    }

    #[test]
    fn read_one_byte() {
        let cursor = Cursor::new(vec![1]);
        assert!(Point::read_from(cursor, 0.into()).is_err());
    }
}
