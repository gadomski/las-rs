//! Write las files.

use std::f64;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use time;

use {Error, Result};
use header::{DEFAULT_BYTES_IN_HEADER, Header, PointFormat, Version};
use point::Point;
use scale::descale;
use vlr::Vlr;

/// A las writer.
#[derive(Debug)]
pub struct Writer<W: Write> {
    auto_offsets: bool,
    header: Header,
    freeze_header: bool,
    writer: W,
    vlrs: Vec<Vlr>,
}

impl Writer<BufWriter<File>> {
    /// Creates a new writer that will write las data to the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Writer<BufWriter<File>>> {
        Ok(Writer::new(BufWriter::new(try!(File::create(path)))))
    }
}

impl<W: Seek + Write> Writer<W> {
    /// Creates a new writer for the given `Write` object.
    ///
    /// Consumes the `Write`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use las::Writer;
    /// let file = File::create("/dev/null").unwrap();
    /// let writer = Writer::new(file);
    /// ```
    pub fn new(writer: W) -> Writer<W> {
        Writer {
            auto_offsets: false,
            header: Header::new(),
            freeze_header: false,
            writer: writer,
            vlrs: Vec::new(),
        }
    }

    /// Sets the writer's header all in one go.
    ///
    /// This will discard any incremental changes made ealier via calls to `scale_factors`, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().header(Header::new());
    /// ```
    pub fn header(mut self, header: Header) -> Writer<W> {
        self.header = header;
        self
    }

    /// Sets the freeze header flag.
    ///
    /// If `freeze_header` is true, than this writer will not recalculate *any* new header values
    /// when writing the header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().freeze_header(true);
    /// ```
    pub fn freeze_header(mut self, freeze_header: bool) -> Writer<W> {
        self.freeze_header = freeze_header;
        self
    }

    /// Sets this writer's vlrs.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().vlrs(Vec::new());
    /// ```
    pub fn vlrs(mut self, vlrs: Vec<Vlr>) -> Writer<W> {
        self.vlrs = vlrs;
        self
    }

    /// Sets the scale factors on a writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().scale_factors(0.01, 0.01, 0.01);
    /// ```
    pub fn scale_factors(mut self,
                         x_scale_factor: f64,
                         y_scale_factor: f64,
                         z_scale_factor: f64)
                         -> Writer<W> {
        self.header.x_scale_factor = x_scale_factor;
        self.header.y_scale_factor = y_scale_factor;
        self.header.z_scale_factor = z_scale_factor;
        self
    }

    /// Sets the offset values for a file.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().offsets(1000.0, 2000.0, 100.0);
    /// ```
    pub fn offsets(mut self, x_offset: f64, y_offset: f64, z_offset: f64) -> Writer<W> {
        self.header.x_offset = x_offset;
        self.header.y_offset = y_offset;
        self.header.z_offset = z_offset;
        self
    }

    /// Enables auto-offsetting.
    ///
    /// If auto-offsetting is enabled, this file will set the header offset values to sensible
    /// values before writing anything. This is usually easier than calculating the offsets
    /// yourself.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().auto_offsets(true);
    /// ```
    pub fn auto_offsets(mut self, enable: bool) -> Writer<W> {
        self.auto_offsets = enable;
        self
    }

    /// Sets the las version for this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().version(1, 2);
    /// ```
    pub fn version(mut self, major: u8, minor: u8) -> Writer<W> {
        self.header.version = Version::new(major, minor);
        self
    }

    /// Sets the point format for this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::PointFormat;
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap().point_format(PointFormat(1));
    /// ```
    pub fn point_format(mut self, point_format: PointFormat) -> Writer<W> {
        self.header.point_data_format = point_format;
        self
    }

    /// Opens this writer for writing points.
    ///
    /// This freezes the headers and the vlrs by returning an `OpenWriter`, which can only write
    /// points.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let mut writer = Writer::from_path("/dev/null").unwrap().open().unwrap();
    /// ```
    pub fn open(mut self) -> Result<OpenWriter<W>> {
        let _ = try!(self.writer.seek(SeekFrom::Start(0)));

        let mut bytes = try!(self.write_header()) as u32;
        bytes += try!(self.write_vlrs());
        bytes = self.header.offset_to_point_data - bytes;
        if bytes > 0 {
            try!(self.writer.write_all(&vec![0; bytes as usize][..]));
        }

        Ok(OpenWriter {
            number_of_point_records: 0,
            number_of_points_by_return: [0; 5],
            writer: self,
            x_max: f64::MIN,
            x_min: f64::MAX,
            y_max: f64::MIN,
            y_min: f64::MAX,
            z_max: f64::MIN,
            z_min: f64::MAX,
        })
    }

    /// Consumes this writer and returns the underlying `Write` object.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let file = Writer::from_path("/dev/null").unwrap().into_inner();
    /// ```
    pub fn into_inner(self) -> W {
        self.writer
    }

    fn rewrite_header(&mut self) -> Result<u16> {
        try!(self.writer.seek(SeekFrom::Start(0)));
        self.write_header()
    }

    fn write_header(&mut self) -> Result<u16> {
        let ref mut header = self.header;
        let ref mut writer = self.writer;

        if !self.freeze_header {
            let now = time::now();

            header.file_creation_day_of_year = now.tm_yday as u16;
            header.file_creation_year = now.tm_year as u16;
            header.header_size = DEFAULT_BYTES_IN_HEADER;
            header.offset_to_point_data = header.header_size as u32 +
                                          self.vlrs.iter().fold(0, |a, v| a + v.len());
            header.point_data_record_length = header.point_data_format.record_length();
            header.number_of_variable_length_records = self.vlrs.len() as u32;
        }

        try!(writer.write_all(&header.file_signature));
        try!(writer.write_u16::<LittleEndian>(header.file_source_id));

        let mut global_encoding = 0;
        if header.version.has_gps_time_type() {
            global_encoding |= header.gps_time_type.as_mask();
        }
        try!(writer.write_u16::<LittleEndian>(global_encoding));
        try!(writer.write_u32::<LittleEndian>(header.guid_data_1));
        try!(writer.write_u16::<LittleEndian>(header.guid_data_2));
        try!(writer.write_u16::<LittleEndian>(header.guid_data_3));
        try!(writer.write_all(&header.guid_data_4));
        try!(writer.write_u8(header.version.major));
        try!(writer.write_u8(header.version.minor));
        try!(writer.write_all(&header.system_identifier));
        try!(writer.write_all(&header.generating_software));
        try!(writer.write_u16::<LittleEndian>(header.file_creation_day_of_year));
        try!(writer.write_u16::<LittleEndian>(header.file_creation_year));
        try!(writer.write_u16::<LittleEndian>(header.header_size));
        try!(writer.write_u32::<LittleEndian>(header.offset_to_point_data));
        try!(writer.write_u32::<LittleEndian>(header.number_of_variable_length_records));
        try!(writer.write_u8(header.point_data_format.0));
        try!(writer.write_u16::<LittleEndian>(header.point_data_record_length));
        try!(writer.write_u32::<LittleEndian>(header.number_of_point_records));
        for n in &header.number_of_points_by_return {
            try!(writer.write_u32::<LittleEndian>(*n));
        }
        try!(writer.write_f64::<LittleEndian>(header.x_scale_factor));
        try!(writer.write_f64::<LittleEndian>(header.y_scale_factor));
        try!(writer.write_f64::<LittleEndian>(header.z_scale_factor));
        try!(writer.write_f64::<LittleEndian>(header.x_offset));
        try!(writer.write_f64::<LittleEndian>(header.y_offset));
        try!(writer.write_f64::<LittleEndian>(header.z_offset));
        try!(writer.write_f64::<LittleEndian>(header.x_max));
        try!(writer.write_f64::<LittleEndian>(header.x_min));
        try!(writer.write_f64::<LittleEndian>(header.y_max));
        try!(writer.write_f64::<LittleEndian>(header.y_min));
        try!(writer.write_f64::<LittleEndian>(header.z_max));
        try!(writer.write_f64::<LittleEndian>(header.z_min));

        let bytes = header.header_size - DEFAULT_BYTES_IN_HEADER;
        if bytes > 0 {
            try!(writer.write_all(&vec![0; bytes as usize][..]));
        }

        Ok(header.header_size)
    }

    fn write_vlrs(&mut self) -> Result<u32> {
        let mut bytes = 0;
        for vlr in &self.vlrs {
            try!(self.writer.write_u16::<LittleEndian>(vlr.reserved));
            try!(self.writer.write_all(&vlr.user_id));
            try!(self.writer.write_u16::<LittleEndian>(vlr.record_id));
            try!(self.writer.write_u16::<LittleEndian>(vlr.record_length_after_header));
            try!(self.writer.write_all(&vlr.description));
            try!(self.writer.write_all(&vlr.record[..]));
            bytes += vlr.len();
        }
        Ok(bytes)
    }

    fn write_point(&mut self, point: &Point) -> Result<()> {
        try!(self.writer.write_i32::<LittleEndian>(descale(point.x,
                                                           self.header.x_scale_factor,
                                                           self.header.x_offset)));
        try!(self.writer.write_i32::<LittleEndian>(descale(point.y,
                                                           self.header.y_scale_factor,
                                                           self.header.y_offset)));
        try!(self.writer.write_i32::<LittleEndian>(descale(point.z,
                                                           self.header.z_scale_factor,
                                                           self.header.z_offset)));
        try!(self.writer.write_u16::<LittleEndian>(point.intensity));
        let byte = point.return_number.as_u8() + (point.number_of_returns.as_u8() << 3) +
                   (point.scan_direction.as_u8() << 6) +
                   ((point.edge_of_flight_line as u8) << 7);
        try!(self.writer.write_u8(byte));
        let byte = point.classification.as_u8() + ((point.synthetic as u8) << 5) +
                   ((point.key_point as u8) << 6) +
                   ((point.withheld as u8) << 7);
        try!(self.writer.write_u8(byte));
        try!(self.writer.write_i8(point.scan_angle_rank));
        try!(self.writer.write_u8(point.user_data));
        try!(self.writer.write_u16::<LittleEndian>(point.point_source_id));
        if self.header.point_data_format.has_time() {
            match point.gps_time {
                Some(gps_time) => try!(self.writer.write_f64::<LittleEndian>(gps_time)),
                None => {
                    return Err(Error::PointFormat(self.header.point_data_format,
                                                  "gps_time".to_string()))
                }
            }
        }
        if self.header.point_data_format.has_color() {
            match point.red {
                Some(red) => try!(self.writer.write_u16::<LittleEndian>(red)),
                None => {
                    return Err(Error::PointFormat(self.header.point_data_format, "red".to_string()))
                }
            }
            match point.green {
                Some(green) => try!(self.writer.write_u16::<LittleEndian>(green)),
                None => {
                    return Err(Error::PointFormat(self.header.point_data_format,
                                                  "green".to_string()))
                }
            }
            match point.blue {
                Some(blue) => try!(self.writer.write_u16::<LittleEndian>(blue)),
                None => {
                    return Err(Error::PointFormat(self.header.point_data_format,
                                                  "blue".to_string()))
                }
            }
        }
        match point.extra_bytes {
            Some(ref bytes) => try!(self.writer.write_all(&bytes[..])),
            None => {}
        }
        Ok(())
    }
}

/// An open writer.
///
/// This writer can only write points. It cannot modify the header or vlrs.
#[derive(Debug)]
pub struct OpenWriter<W: Write> {
    number_of_point_records: u32,
    number_of_points_by_return: [u32; 5],
    writer: Writer<W>,
    x_max: f64,
    x_min: f64,
    y_max: f64,
    y_min: f64,
    z_max: f64,
    z_min: f64,
}

impl<W: Seek + Write> OpenWriter<W> {
    /// Writes a single point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Point, Writer};
    /// let mut writer = Writer::from_path("/dev/null").unwrap().open().unwrap();
    /// writer.write_point(&Point::new()).unwrap();
    /// ```
    pub fn write_point(&mut self, point: &Point) -> Result<()> {
        self.number_of_point_records += 1;
        if point.return_number.as_u8() > 0 {
            self.number_of_points_by_return[point.return_number.as_u8() as usize - 1] += 1;
        }
        if point.x < self.x_min {
            self.x_min = point.x;
        }
        if point.y < self.y_min {
            self.y_min = point.y;
        }
        if point.z < self.z_min {
            self.z_min = point.z;
        }
        if point.x > self.x_max {
            self.x_max = point.x;
        }
        if point.y > self.y_max {
            self.y_max = point.y;
        }
        if point.z > self.z_max {
            self.z_max = point.z;
        }
        self.writer.write_point(point)
    }

    /// Writes several points to this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// Writer::from_path("/dev/null").unwrap().open().unwrap()
    ///     .write_points(&Vec::new()).unwrap();
    /// ```
    pub fn write_points(&mut self, points: &Vec<Point>) -> Result<()> {
        for point in points {
            try!(self.write_point(point));
        }
        Ok(())
    }

    /// Closes this open writer.
    ///
    /// The close operation will update some fields in the header based upon the statistics of the
    /// points that were written.
    ///
    /// This method consumes the open writer and returns the original `Writer` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap()
    ///     .open().unwrap()
    ///     .close().unwrap();
    /// ```
    pub fn close(mut self) -> Result<Writer<W>> {
        if !self.writer.freeze_header {
            self.writer.header.number_of_point_records = self.number_of_point_records;
            self.writer.header.number_of_points_by_return = self.number_of_points_by_return;
            self.writer.header.x_max = self.x_max;
            self.writer.header.x_min = self.x_min;
            self.writer.header.y_max = self.y_max;
            self.writer.header.y_min = self.y_min;
            self.writer.header.z_max = self.z_max;
            self.writer.header.z_min = self.z_min;
            try!(self.writer.rewrite_header());
        }
        Ok(self.writer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    use header::PointFormat;
    use point::{Point, ReturnNumber, NumberOfReturns};
    use reader::Reader;

    #[test]
    fn builder() {
        let writer = Writer::new(Cursor::new(Vec::new()))
            .scale_factors(1.0, 2.0, 3.0)
            .offsets(4.0, 5.0, 6.0)
            .version(1, 2)
            .point_format(PointFormat(1))
            .open()
            .unwrap()
            .close()
            .unwrap();

        let mut cursor = writer.into_inner();
        cursor.set_position(0);
        let reader = Reader::new(cursor).unwrap();
        let header = reader.header();
        assert_eq!(1.0, header.x_scale_factor);
        assert_eq!(2.0, header.y_scale_factor);
        assert_eq!(3.0, header.z_scale_factor);
        assert_eq!(4.0, header.x_offset);
        assert_eq!(5.0, header.y_offset);
        assert_eq!(6.0, header.z_offset);
        assert_eq!(1, header.version.major);
        assert_eq!(2, header.version.minor);
        assert_eq!(PointFormat(1), header.point_data_format);
    }

    #[test]
    fn write_one_point() {
        let mut point = Point::new();
        point.x = 1.0;
        point.y = 2.0;
        point.z = 3.0;
        point.return_number = ReturnNumber::from_u8(2).unwrap();
        point.number_of_returns = NumberOfReturns::from_u8(3).unwrap();
        let mut writer = Writer::new(Cursor::new(Vec::new())).open().unwrap();
        writer.write_point(&point).unwrap();
        let mut cursor = writer.close().unwrap().into_inner();
        cursor.set_position(0);

        let mut reader = Reader::new(cursor).unwrap();
        let &header = reader.header();
        assert_eq!(1, header.number_of_point_records);
        assert_eq!([0, 1, 0, 0, 0], header.number_of_points_by_return);
        assert_eq!(1.0, header.x_max);
        assert_eq!(1.0, header.x_min);
        assert_eq!(2.0, header.y_max);
        assert_eq!(2.0, header.y_min);
        assert_eq!(3.0, header.z_max);
        assert_eq!(3.0, header.z_min);

        let point = reader.read_point().unwrap().unwrap();
        assert_eq!(1.0, point.x);
        assert_eq!(2.0, point.y);
        assert_eq!(3.0, point.z);
    }
}
