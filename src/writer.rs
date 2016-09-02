//! Write points to a `Write`.
//!
//! Simple writes can be done with a `Writer`, but if you need to configure your output file, use a
//! `Builder`.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use chrono::Datelike;

use {Error, Result};
use global_encoding::{GlobalEncoding, GpsTime};
use header::Header;
use point::{Color, Format, Point, utils};
use reader::Reader;
use utils::{Bounds, Triple};
use version::Version;

/// Configure a `Writer`.
#[derive(Debug)]
pub struct Builder {
    header: Header,
}

impl Builder {
    /// Creates a new `Builder`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let builder = Builder::new();
    /// ```
    pub fn new() -> Builder {
        Builder { header: Header::default() }
    }

    /// Creates a new `Builder` and configures it to match the provided `Reader`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// use las::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let builder = Builder::from_reader(&reader);
    /// ```
    pub fn from_reader<R>(reader: &Reader<R>) -> Builder {
        Builder { header: reader.header.clone() }
    }

    /// Sets the file source id.
    ///
    /// This field was added in LAS 1.1.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let builder = Builder::new().file_source_id(1);
    /// ```
    pub fn file_source_id(mut self, file_source_id: u16) -> Builder {
        self.header.file_source_id = Some(file_source_id);
        self
    }

    /// Sets the global encoding.
    ///
    /// This field was added in LAS 1.1.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// use las::global_encoding::GlobalEncoding;
    /// let builder = Builder::new().global_encoding(GlobalEncoding::from(1));
    /// ```
    pub fn global_encoding(mut self, global_encoding: GlobalEncoding) -> Builder {
        self.header.global_encoding = Some(global_encoding);
        self
    }

    /// Sets the LAS version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// use las::Version;
    /// let builder = Builder::new().version(Version::new(1, 2));
    /// ```
    pub fn version(mut self, version: Version) -> Builder {
        self.header.version = version;
        self
    }

    /// Sets the point format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// use las::point;
    /// let builder = Builder::new().point_format(point::Format::from(1));
    /// ```
    pub fn point_format(mut self, format: Format) -> Builder {
        self.header.point_format = format;
        self
    }

    /// Sets the extra bytes on the output file.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let builder = Builder::new().extra_bytes(2);
    /// ```
    pub fn extra_bytes(mut self, extra_bytes: u16) -> Builder {
        self.header.extra_bytes = extra_bytes;
        self
    }

    /// Creates a `Writer`.
    ///
    /// This method does *not* consume the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Builder;
    /// let writer = Builder::new().writer(Cursor::new(Vec::new())).unwrap();
    /// ```
    pub fn writer<W: Seek + Write>(&self, write: W) -> Result<Writer<W>> {
        Writer::new(self, write)
    }

    /// Creates a `Writer` that will write out data to the path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let writer = Builder::new().writer_from_path("/dev/null").unwrap();
    /// ```
    pub fn writer_from_path<P: AsRef<Path>>(&self, path: P) -> Result<Writer<BufWriter<File>>> {
        File::create(path).map_err(Error::from).and_then(|f| self.writer(BufWriter::new(f)))
    }
}

impl Default for Builder {
    fn default() -> Builder {
        Builder::new()
    }
}

/// Write LAS points to a `Write`.
///
/// This struct implements `Drop`, so the LAS data are finalized (the header is re-written) when
/// the `Writer` goes out of scope. This will panic if there is an error while closing the file, so
/// if you're worried about panics you will need to use `Writer::close` instead.
#[derive(Debug)]
pub struct Writer<W: Seek + Write> {
    bounds: Bounds<f64>,
    closed: bool,
    header: Header,
    point_count: u32,
    point_count_by_return: [u32; 5],
    write: W,
}

impl Writer<BufWriter<File>> {
    /// Creates a default `Writer` that will write points out to a file at the path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Writer;
    /// let writer = Writer::from_path("/dev/null").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Writer<BufWriter<File>>> {
        File::create(path).map_err(Error::from).and_then(|f| Writer::default(BufWriter::new(f)))
    }
}

impl<W: Seek + Write> Writer<W> {
    fn from(builder: &Builder, write: W) -> Writer<W> {
        Writer {
            bounds: Default::default(),
            closed: false,
            header: builder.header.clone(),
            point_count: 0,
            point_count_by_return: [0; 5],
            write: write,
        }
    }

    fn new(builder: &Builder, write: W) -> Result<Writer<W>> {
        let mut writer = Writer::from(builder, write);
        try!(writer.write_header());
        Ok(writer)
    }

    /// Creates a new default writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let writer = Writer::default(Cursor::new(Vec::new())).unwrap();
    /// ```
    pub fn default(write: W) -> Result<Writer<W>> {
        Builder::new().writer(write)
    }

    /// Writes a point.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
    /// writer.write(Default::default()).unwrap();
    /// ```
    pub fn write(&mut self, point: Point) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }
        try!(self.write
            .write_i32::<LittleEndian>(((point.x - self.header.offset.x) /
                                        self.header
                    .scale
                    .x)
                .round() as i32));
        try!(self.write
            .write_i32::<LittleEndian>(((point.y - self.header.offset.y) /
                                        self.header
                    .scale
                    .y)
                .round() as i32));
        try!(self.write
            .write_i32::<LittleEndian>(((point.z - self.header.offset.z) /
                                        self.header
                    .scale
                    .z)
                .round() as i32));
        try!(self.write.write_u16::<LittleEndian>(point.intensity));
        try!(self.write
            .write_u8(u8::from(point.return_number) | u8::from(point.number_of_returns) |
                      u8::from(point.scan_direction) |
                      utils::edge_of_flight_line_u8(point.edge_of_flight_line)));
        try!(self.write.write_u8(point.classification.into()));
        try!(self.write.write_i8(point.scan_angle_rank));
        try!(self.write.write_u8(point.user_data));
        try!(self.write.write_u16::<LittleEndian>(point.point_source_id));
        if self.header.point_format.has_gps_time() {
            match point.gps_time {
                Some(time) => try!(self.write.write_f64::<LittleEndian>(time)),
                None => return Err(Error::MissingGpsTime(self.header.point_format, point)),
            }
        }
        if self.header.point_format.has_color() {
            match point.color {
                Some(Color { red, green, blue }) => {
                    try!(self.write.write_u16::<LittleEndian>(red));
                    try!(self.write.write_u16::<LittleEndian>(green));
                    try!(self.write.write_u16::<LittleEndian>(blue));
                }
                None => return Err(Error::MissingColor(self.header.point_format, point)),
            }
        }
        if self.header.extra_bytes > 0 {
            try!(self.write.write_all(point.extra_bytes.as_slice()));
        }
        self.point_count += 1;
        if point.return_number.is_valid() {
            self.point_count_by_return[u8::from(point.return_number) as usize - 1] += 1;
        }
        self.bounds.grow(Triple {
            x: point.x,
            y: point.y,
            z: point.z,
        });
        Ok(())
    }

    fn write_header(&mut self) -> Result<()> {
        let header = &self.header;
        try!(self.write.seek(SeekFrom::Start(0)));
        try!(self.write.write(b"LASF"));
        let file_source_id = if header.version.has_file_source_id() {
            if let Some(file_source_id) = header.file_source_id {
                file_source_id
            } else {
                info!("Writer doesn't have a file source id, writing zero");
                0
            }
        } else {
            if header.file_source_id.is_some() {
                warn!("Version {} does not support file source id, writing zero instead",
                      header.version);
            }
            0
        };
        try!(self.write.write_u16::<LittleEndian>(file_source_id));
        let global_encoding = if header.version.has_global_encoding() {
            if let Some(global_encoding) = header.global_encoding {
                global_encoding.into()
            } else {
                info!("Writer doesn't have a global encoding, writing zero");
                0
            }
        } else {
            if let Some(global_encoding) = header.global_encoding {
                match global_encoding.gps_time {
                    GpsTime::Standard => {
                        return Err(Error::GpsTimeMismatch(header.version, GpsTime::Standard))
                    }
                    _ => {}
                };
            }
            0
        };
        try!(self.write.write_u16::<LittleEndian>(global_encoding));
        try!(self.write.write(&header.project_id));
        try!(self.write.write_u8(header.version.major));
        try!(self.write.write_u8(header.version.minor));
        try!(self.write.write(&header.system_id));
        try!(self.write.write(&header.generating_software));
        try!(self.write.write_u16::<LittleEndian>(header.file_creation_date.ordinal() as u16));
        try!(self.write.write_u16::<LittleEndian>(header.file_creation_date.year() as u16));
        try!(self.write.write_u16::<LittleEndian>(header.header_size));
        try!(self.write
            .write_u32::<LittleEndian>(header.vlrs
                .iter()
                .fold(header.padding + header.header_size as u32,
                      |acc, vlr| acc + vlr.len())));
        try!(self.write.write_u32::<LittleEndian>(header.vlrs.len() as u32));
        try!(self.write.write_u8(header.point_format.into()));
        try!(self.write
            .write_u16::<LittleEndian>(header.point_format.record_length() + header.extra_bytes));
        try!(self.write.write_u32::<LittleEndian>(self.point_count));
        for &count in &self.point_count_by_return {
            try!(self.write.write_u32::<LittleEndian>(count));
        }
        try!(self.write.write_f64::<LittleEndian>(header.scale.x));
        try!(self.write.write_f64::<LittleEndian>(header.scale.y));
        try!(self.write.write_f64::<LittleEndian>(header.scale.z));
        try!(self.write.write_f64::<LittleEndian>(header.offset.x));
        try!(self.write.write_f64::<LittleEndian>(header.offset.y));
        try!(self.write.write_f64::<LittleEndian>(header.offset.z));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.max.x));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.min.x));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.max.y));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.min.y));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.max.z));
        try!(self.write.write_f64::<LittleEndian>(self.bounds.min.z));
        for vlr in &header.vlrs {
            try!(self.write.write_u16::<LittleEndian>(0)); // reserved
            try!(self.write.write(&vlr.user_id));
            try!(self.write.write_u16::<LittleEndian>(vlr.record_id));
            try!(self.write.write_u16::<LittleEndian>(vlr.record_length));
            try!(self.write.write(&vlr.description));
            try!(self.write.write(&vlr.data));
        }
        if header.padding > 0 {
            let padding = vec![0; header.padding as usize];
            try!(self.write.write(&padding));
        }
        Ok(())
    }

    /// Closes this writer.
    ///
    /// After the writer is closed, and future calls to `write` will error.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
    /// assert!(writer.write(Default::default()).is_ok());
    /// writer.close().unwrap();
    /// assert!(writer.write(Default::default()).is_err());
    /// ```
    pub fn close(&mut self) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }
        try!(self.write_header());
        self.closed = true;
        Ok(())
    }
}

impl<W: Seek + Write> Drop for Writer<W> {
    fn drop(&mut self) {
        if !self.closed {
            if let Err(err) = self.close() {
                error!("Error while dropping writer: {}", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::io::{Cursor, Read};

    use global_encoding::GlobalEncoding;
    use point::{Classification, Color, Format, NumberOfReturns, Point, ReturnNumber, ScanDirection};
    use reader::Reader;
    use utils::Bounds;
    use version::Version;

    fn point() -> Point {
        Point {
            x: 1.,
            y: 2.,
            z: 3.,
            intensity: 4,
            return_number: ReturnNumber::from(1),
            number_of_returns: NumberOfReturns::from(0b00010000),
            scan_direction: ScanDirection::Positive,
            edge_of_flight_line: false,
            classification: Classification::from(2, Version::new(1, 2)),
            scan_angle_rank: 2,
            user_data: 3,
            point_source_id: 4,
            gps_time: Some(5.),
            color: Some(Color {
                red: 6,
                green: 7,
                blue: 8,
            }),
            extra_bytes: Vec::new(),
        }
    }

    fn check_point(point: &Point) {
        assert_eq!(1., point.x);
        assert_eq!(2., point.y);
        assert_eq!(3., point.z);
        assert_eq!(4, point.intensity);
        assert_eq!(1, point.return_number);
        assert_eq!(2, point.number_of_returns);
        assert_eq!(ScanDirection::Positive, point.scan_direction);
        assert!(!point.edge_of_flight_line);
        assert_eq!(2, point.classification);
        assert_eq!(2, point.scan_angle_rank);
        assert_eq!(3, point.user_data);
        assert_eq!(4, point.point_source_id);
    }

    fn check_point_0(point: Point) {
        check_point(&point);
        assert!(point.gps_time.is_none());
        assert!(point.color.is_none());
    }

    fn check_point_1(point: Point) {
        check_point(&point);
        assert_eq!(5., point.gps_time.unwrap());
        assert!(point.color.is_none());
    }

    fn check_point_2(point: Point) {
        check_point(&point);
        assert!(point.gps_time.is_none());
        let color = point.color.unwrap();
        assert_eq!(6, color.red);
        assert_eq!(7, color.green);
        assert_eq!(8, color.blue);
    }

    fn check_point_3(point: Point) {
        check_point(&point);
        assert_eq!(5., point.gps_time.unwrap());
        let color = point.color.unwrap();
        assert_eq!(6, color.red);
        assert_eq!(7, color.green);
        assert_eq!(8, color.blue);
    }

    #[test]
    fn write_zero_points() {
        let mut cursor = Cursor::new(Vec::new());
        Writer::default(&mut cursor).unwrap();
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn write_one_point_default() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        check_point_0(reader.read().unwrap().unwrap());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn write_one_point_format_1() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                Builder::new().point_format(Format::from(1)).writer(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        check_point_1(reader.read().unwrap().unwrap());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn write_one_point_format_2() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                Builder::new().point_format(Format::from(2)).writer(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        check_point_2(reader.read().unwrap().unwrap());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn write_one_point_format_3() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer =
                Builder::new().point_format(Format::from(3)).writer(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        check_point_3(reader.read().unwrap().unwrap());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn try_write_point_format_1_no_time() {
        let mut writer =
            Builder::new().point_format(Format::from(1)).writer(Cursor::new(Vec::new())).unwrap();
        let mut point = point();
        point.gps_time = None;
        assert!(writer.write(point).is_err());
    }

    #[test]
    fn try_write_point_format_2_no_color() {
        let mut writer =
            Builder::new().point_format(Format::from(2)).writer(Cursor::new(Vec::new())).unwrap();
        let mut point = point();
        point.color = None;
        assert!(writer.write(point).is_err());
    }

    #[test]
    fn write_point_count() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        assert_eq!(1, Reader::new(cursor).unwrap().header.point_count);
    }

    #[test]
    fn write_two_points() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(point()).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        assert!(reader.read().unwrap().is_some());
        assert!(reader.read().unwrap().is_some());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn write_bounds() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(point()).unwrap();
        }
        cursor.set_position(0);
        let reader = Reader::new(cursor).unwrap();
        assert_eq!(Bounds::new(1., 2., 3., 1., 2., 3.), reader.header.bounds);
    }

    #[test]
    fn close_the_writer() {
        let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
        writer.close().is_ok();
        assert!(writer.write(point()).is_err());
        assert!(writer.close().is_err());
    }

    #[test]
    fn write_version() {
        let mut cursor = Cursor::new(Vec::new());
        Builder::new().version(Version::new(1, 0)).writer(&mut cursor).unwrap();
        cursor.set_position(0);
        assert_eq!(Version::new(1, 0),
                   Reader::new(&mut cursor).unwrap().header.version);
    }

    #[test]
    fn write_bitwise_exact() {
        let mut buffer = Vec::new();
        File::open("data/1.0_0.las").unwrap().read_to_end(&mut buffer).unwrap();
        let mut original = Cursor::new(buffer);
        let mut secondary = Cursor::new(Vec::new());
        {
            let mut reader = Reader::new(&mut original).unwrap();
            let mut writer = Builder::from_reader(&reader).writer(&mut secondary).unwrap();
            for point in reader.iter_mut() {
                writer.write(point.unwrap()).unwrap();
            }
        }
        let original = original.into_inner();
        let secondary = secondary.into_inner();
        assert_eq!(original.len(), secondary.len());
    }

    #[test]
    fn write_zero_return_number() {
        let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
        writer.write(Default::default()).unwrap();
    }

    #[test]
    fn write_extra_bytes() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Builder::new().extra_bytes(5).writer(&mut cursor).unwrap();
            let mut point = point();
            point.extra_bytes = b"Hello".to_vec();
            writer.write(point).unwrap();
        }
        cursor.set_position(0);
        let point = Reader::new(cursor).unwrap().read_to_end().unwrap().pop().unwrap();
        assert_eq!(b"Hello", &point.extra_bytes[..]);
    }

    #[test]
    fn writer_from_path() {
        assert!(Writer::from_path("/dev/null").is_ok());
    }

    #[test]
    fn builder_writer_for_path() {
        assert!(Builder::new().writer_from_path("/dev/null").is_ok());
    }

    #[test]
    fn wipe_filesource_id() {
        let mut cursor = Cursor::new(Vec::new());
        Builder::new().file_source_id(1).version(Version::new(1, 0)).writer(&mut cursor).unwrap();
        cursor.set_position(0);
        assert!(Reader::new(cursor).is_ok());
    }

    #[test]
    fn disallow_global_encoding_downcast() {
        assert!(Builder::new()
            .global_encoding(GlobalEncoding::from(1))
            .version(Version::new(1, 0))
            .writer(Cursor::new(Vec::new()))
            .is_err());
    }
}
