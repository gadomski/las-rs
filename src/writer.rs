//! Write points to a `Write`.
//!
//! Simple writes can be done with a `Writer`, but if you need to configure your output file, use a
//! `Builder`.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use {Error, Result};
use builder::Builder;
use header::{Header, WriteHeader};
use point::{Point, WritePoint};
use utils::{Bounds, Triple};
use vlr::{Vlr, WriteVlr};

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
    vlrs: Vec<Vlr>,
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
            header: builder.into(),
            point_count: Default::default(),
            point_count_by_return: Default::default(),
            vlrs: builder.vlrs.clone(),
            write: write,
        }
    }

    /// Creates a new writer from a `Builder` and a `Write`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Writer;
    /// use std::io::Cursor;
    /// use las::Builder;
    /// let writer = Writer::new(&Builder::new(), Cursor::new(Vec::new())).unwrap();
    /// ```
    pub fn new(builder: &Builder, write: W) -> Result<Writer<W>> {
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
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
    /// writer.write(&Default::default()).unwrap();
    /// ```
    pub fn write(&mut self, point: &Point) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }
        try!(self.write.write_point(point,
                                    self.header.transforms,
                                    self.header.point_format,
                                    self.header.extra_bytes));
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
        // TODO test point count by return, offsets, etc
        self.header.point_count = self.point_count;
        self.header.bounds = self.bounds;
        try!(self.write.seek(SeekFrom::Start(0)));
        try!(self.write.write_header(self.header));
        for vlr in &self.vlrs {
            try!(self.write.write_vlr(vlr));
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
    /// assert!(writer.write(&Default::default()).is_ok());
    /// writer.close().unwrap();
    /// assert!(writer.write(&Default::default()).is_err());
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

    use std::io::Cursor;

    use builder::Builder;
    use point::{Classification, Color, NumberOfReturns, Point, ReturnNumber, ScanDirection};
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
            classification: Classification::from(2),
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
            writer.write(&point()).unwrap();
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
            let mut builder = Builder::new();
            builder.point_format = 1.into();
            let mut writer = builder.writer(&mut cursor).unwrap();
            writer.write(&point()).unwrap();
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
            let mut builder = Builder::new();
            builder.point_format = 2.into();
            let mut writer = builder.writer(&mut cursor).unwrap();
            writer.write(&point()).unwrap();
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
            let mut builder = Builder::new();
            builder.point_format = 3.into();
            let mut writer = builder.writer(&mut cursor).unwrap();
            writer.write(&point()).unwrap();
        }
        cursor.set_position(0);
        let mut reader = Reader::new(cursor).unwrap();
        check_point_3(reader.read().unwrap().unwrap());
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn try_write_point_format_1_no_time() {
        let mut builder = Builder::new();
        builder.point_format = 1.into();
        let mut writer = builder.writer(Cursor::new(Vec::new())).unwrap();
        let mut point = point();
        point.gps_time = None;
        assert!(writer.write(&point).is_err());
    }

    #[test]
    fn try_write_point_format_2_no_color() {
        let mut builder = Builder::new();
        builder.point_format = 2.into();
        let mut writer = builder.writer(Cursor::new(Vec::new())).unwrap();
        let mut point = point();
        point.color = None;
        assert!(writer.write(&point).is_err());
    }

    #[test]
    fn write_point_count() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(&point()).unwrap();
        }
        cursor.set_position(0);
        assert_eq!(1, Reader::new(cursor).unwrap().header.point_count);
    }

    #[test]
    fn write_two_points() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = Writer::default(&mut cursor).unwrap();
            writer.write(&point()).unwrap();
            writer.write(&point()).unwrap();
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
            writer.write(&point()).unwrap();
        }
        cursor.set_position(0);
        let reader = Reader::new(cursor).unwrap();
        assert_eq!(Bounds::new(1., 2., 3., 1., 2., 3.), reader.header.bounds);
    }

    #[test]
    fn close_the_writer() {
        let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
        writer.close().is_ok();
        assert!(writer.write(&point()).is_err());
        assert!(writer.close().is_err());
    }

    #[test]
    fn write_version() {
        let mut cursor = Cursor::new(Vec::new());
        let mut builder = Builder::new();
        builder.version = (1, 0).into();
        builder.writer(&mut cursor).unwrap();
        cursor.set_position(0);
        assert_eq!(Version::from((1, 0)),
                   Reader::new(&mut cursor).unwrap().header.version);
    }

    #[test]
    fn write_zero_return_number() {
        let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
        writer.write(&Default::default()).unwrap();
    }

    #[test]
    fn write_extra_bytes() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut builder = Builder::new();
            builder.extra_bytes = 5;
            let mut writer = builder.writer(&mut cursor).unwrap();
            let mut point = point();
            point.extra_bytes = b"Hello".to_vec();
            writer.write(&point).unwrap();
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
    fn disallow_file_source_id_wipe() {
        let mut builder = Builder::new();
        builder.file_source_id = 1;
        builder.version = (1, 0).into();
        assert!(builder.writer(Cursor::new(Vec::new())).is_err());
    }

    #[test]
    fn disallow_global_encoding_downcast() {
        let mut builder = Builder::new();
        builder.global_encoding = 1.into();
        builder.version = (1, 0).into();
        assert!(builder.writer(Cursor::new(Vec::new())).is_err());
    }
}
