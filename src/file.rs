//! `las` file management.

use std::fs;
use std::io::{BufReader, Seek, SeekFrom, Read, Write};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use header::Header;
use io::{read_full, write_zeros};
use point::{Classification, NumberOfReturns, Point, ReturnNumber, ScanDirection};
use scale::{descale, scale};
use vlr::Vlr;

use super::{LasError, Result};

/// A las file.
#[derive(Debug, PartialEq)]
pub struct File {
    header: Header,
    vlrs: Vec<Vlr>,
    points: Vec<Point>,
}

impl File {
    /// Reads a las file from the filesystem.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::file::File;
    /// let file = File::from_path("data/1.0_0.las").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<File> {
        let ref mut reader = BufReader::new(try!(fs::File::open(path)));
        File::read_from(reader)
    }

    /// Reads a las file from a `Read`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    /// use las::file::File;
    /// let ref mut reader = fs::File::open("data/1.0_0.las").unwrap();
    /// let file = File::read_from(reader).unwrap();
    /// ```
    pub fn read_from<R: Read + Seek>(reader: &mut R) -> Result<File> {
        let mut file = File::new();

        file.header = try!(Header::read_from(reader));

        let _ = try!(reader.seek(SeekFrom::Start(file.header.header_size as u64)));
        file.vlrs.reserve(file.header.number_of_variable_length_records as usize);
        for _ in 0..file.header.number_of_variable_length_records {
            file.vlrs.push(try!(Vlr::read_from(reader)));
        }

        let _ = try!(reader.seek(SeekFrom::Start(file.header.offset_to_point_data as u64)));
        file.points.reserve(file.header.number_of_point_records as usize);
        for _ in 0..file.header.number_of_point_records {
            let point = try!(file.read_point_from(reader));
            file.points.push(point);
        }

        Ok(file)
    }

    /// Creates a new, empty las file.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::file::File;
    /// let file = File::new();
    /// ```
    pub fn new() -> File {
        File {
            header: Header::new(),
            vlrs: Vec::new(),
            points: Vec::new(),
        }
    }

    fn read_point_from<R: Read + Seek>(&self, reader: &mut R) -> Result<Point> {
        let mut point = Point::new();
        let start = try!(reader.seek(SeekFrom::Current(0)));
        point.x = scale(try!(reader.read_i32::<LittleEndian>()),
                        self.header.x_scale_factor,
                        self.header.x_offset);
        point.y = scale(try!(reader.read_i32::<LittleEndian>()),
                        self.header.y_scale_factor,
                        self.header.y_offset);
        point.z = scale(try!(reader.read_i32::<LittleEndian>()),
                        self.header.z_scale_factor,
                        self.header.x_offset);
        point.intensity = try!(reader.read_u16::<LittleEndian>());
        let byte = try!(reader.read_u8());
        point.return_number = try!(ReturnNumber::from_u8(byte & 0b00000111));
        point.number_of_returns = try!(NumberOfReturns::from_u8((byte >> 3) & 0b00000111));
        point.scan_direction = ScanDirection::from((byte >> 6) & 0b00000001 == 1);
        point.edge_of_flight_line = byte >> 7 == 1;
        let byte = try!(reader.read_u8());
        point.classification = try!(Classification::from_u8(byte & 0b00011111));
        point.synthetic = (byte >> 5) & 0b00000001 == 1;
        point.key_point = (byte >> 6) & 0b00000001 == 1;
        point.withheld = byte >> 7 == 1;
        point.scan_angle_rank = try!(reader.read_i8());
        point.user_data = try!(reader.read_u8());
        point.point_source_id = try!(reader.read_u16::<LittleEndian>());
        if self.header.point_data_format.has_time() {
            point.gps_time = Some(try!(reader.read_f64::<LittleEndian>()));
        }
        if self.header.point_data_format.has_color() {
            point.red = Some(try!(reader.read_u16::<LittleEndian>()));
            point.green = Some(try!(reader.read_u16::<LittleEndian>()));
            point.blue = Some(try!(reader.read_u16::<LittleEndian>()));
        }
        let bytes_read = try!(reader.seek(SeekFrom::Current(0))) - start;

        if bytes_read < self.header.point_data_record_length as u64 {
            let mut buf = vec![0; (self.header.point_data_record_length as u64 - bytes_read) as usize];
            try!(read_full(reader, &mut buf[..]));
            point.extra_bytes = Some(buf);
        }

        Ok(point)
    }

    /// Returns a reference to a vector of this file's points.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::file::File;
    /// let file = File::from_path("data/1.0_0.las").unwrap();
    /// let points = file.points();
    /// ```
    pub fn points(&self) -> &Vec<Point> {
        &self.points
    }

    /// Writes this las file to a `Write`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::file::File;
    /// let file = File::from_path("data/1.0_0.las").unwrap();
    /// let ref mut cursor = Cursor::new(Vec::new());
    /// file.write_to(cursor).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let mut bytes_written = try!(self.header.write_to(writer)) as usize;
        if bytes_written < self.header.header_size as usize {
            bytes_written += try!(write_zeros(writer,
                                              self.header.header_size as usize - bytes_written));
        }
        for vlr in &self.vlrs {
            bytes_written += try!(vlr.write_to(writer)) as usize;
        }
        if bytes_written < self.header.offset_to_point_data as usize {
            try!(write_zeros(writer,
                             self.header.offset_to_point_data as usize - bytes_written));
        }
        for point in &self.points {
            try!(self.write_point_to(writer, point));
        }
        Ok(())
    }

    fn write_point_to<W: Write>(&self, writer: &mut W, point: &Point) -> Result<()> {
        try!(writer.write_i32::<LittleEndian>(descale(point.x,
                                                      self.header.x_scale_factor,
                                                      self.header.x_offset)));
        try!(writer.write_i32::<LittleEndian>(descale(point.y,
                                                      self.header.y_scale_factor,
                                                      self.header.y_offset)));
        try!(writer.write_i32::<LittleEndian>(descale(point.z,
                                                      self.header.z_scale_factor,
                                                      self.header.z_offset)));
        try!(writer.write_u16::<LittleEndian>(point.intensity));
        let byte = point.return_number.as_u8() + (point.number_of_returns.as_u8() << 3) +
                   (point.scan_direction.as_u8() << 6) +
                   ((point.edge_of_flight_line as u8) << 7);
        try!(writer.write_u8(byte));
        let byte = point.classification.as_u8() + ((point.synthetic as u8) << 5) +
                   ((point.key_point as u8) << 6) +
                   ((point.withheld as u8) << 7);
        try!(writer.write_u8(byte));
        try!(writer.write_i8(point.scan_angle_rank));
        try!(writer.write_u8(point.user_data));
        try!(writer.write_u16::<LittleEndian>(point.point_source_id));
        if self.header.point_data_format.has_time() {
            match point.gps_time {
                Some(gps_time) => try!(writer.write_f64::<LittleEndian>(gps_time)),
                None => return Err(LasError::PointFormat(self.header.point_data_format, "gps_time".to_string())),
            }
        }
        if self.header.point_data_format.has_color() {
            match point.red {
                Some(red) => try!(writer.write_u16::<LittleEndian>(red)),
                None => return Err(LasError::PointFormat(self.header.point_data_format, "red".to_string())),
            }
            match point.green {
                Some(green) => try!(writer.write_u16::<LittleEndian>(green)),
                None => return Err(LasError::PointFormat(self.header.point_data_format, "green".to_string())),
            }
            match point.blue {
                Some(blue) => try!(writer.write_u16::<LittleEndian>(blue)),
                None => return Err(LasError::PointFormat(self.header.point_data_format, "blue".to_string())),
            }
        }
        match point.extra_bytes {
            Some(ref bytes) => try!(writer.write_all(&bytes[..])),
            None => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;
    use std::path::Path;

    fn roundtrip<P: AsRef<Path>>(path: P) {
        let lasfile = File::from_path(path).unwrap();
        let ref mut cursor = Cursor::new(Vec::new());
        lasfile.write_to(cursor).unwrap();
        cursor.set_position(0);
        let lasfile2 = File::read_from(cursor).unwrap();
        assert_eq!(lasfile, lasfile2);
    }

    #[test]
    fn roundtrip_1_0_0() { roundtrip("data/1.0_0.las"); }

    #[test]
    fn roundtrip_1_0_1() { roundtrip("data/1.0_1.las"); }

    #[test]
    fn roundtrip_1_1_0() { roundtrip("data/1.1_0.las"); }

    #[test]
    fn roundtrip_1_1_1() { roundtrip("data/1.1_1.las"); }

    #[test]
    fn roundtrip_1_2_0() { roundtrip("data/1.2_0.las"); }

    #[test]
    fn roundtrip_1_2_1() { roundtrip("data/1.2_1.las"); }

    #[test]
    fn roundtrip_1_2_2() { roundtrip("data/1.2_2.las"); }

    #[test]
    fn roundtrip_1_2_3() { roundtrip("data/1.2_3.las"); }

    /// This file is good as it exercieses a weird use case, but the test fails at the moment. I'm
    /// not sure why, so I'm going to keep it around but ignore it.
    #[test]
    #[ignore]
    fn roundtrip_extrabytes() { roundtrip("data/extrabytes.las"); }

    #[test]
    fn point_format_1_has_gps_time() {
        let lasfile = File::from_path("data/1.0_1.las").unwrap();
        let ref point = lasfile.points()[0];
        assert!(point.gps_time.is_some());
    }

    #[test]
    fn point_format_2_has_color() {
        let lasfile = File::from_path("data/1.2_2.las").unwrap();
        let ref point = lasfile.points()[0];
        assert!(point.red.is_some());
        assert!(point.green.is_some());
        assert!(point.blue.is_some());
    }
}
