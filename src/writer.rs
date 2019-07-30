//! Write las points.
//!
//! A `StdWriter` uses a `Header` for its configuration:
//!
//! ```
//! use std::io::Cursor;
//! use las::{StdWriter, Header};
//! let mut header = Header::from((1, 4));
//! let writer = StdWriter::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! The set of optional fields on the point format and the points must match exactly:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Builder, StdWriter, Writer, Point};
//! use las::point::Format;
//! use las::Color;
//!
//! let mut builder = Builder::default();
//! builder.point_format = Format::new(1).unwrap();
//! let mut writer = StdWriter::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap();
//!
//! let mut point = Point::default(); // default points don't have any optional attributes
//! assert!(writer.write(point.clone()).is_err());
//!
//! point.gps_time = Some(42.); // point format 1 requires gps time
//! writer.write(point.clone()).unwrap();
//!
//! point.color = Some(Color::new(1, 2, 3));
//! assert!(writer.write(point).is_err()); // the point's color would be lost
//! ```

use std::fmt::Debug;
use std::fs::File;
use std::io::{BufWriter, Cursor, Seek, SeekFrom, Write};
use std::path::Path;

#[cfg(feature = "laz")]
use compression::CompressedPointWriter;

use point::Format;
use {Header, Point, Result};

quick_error! {
    /// Writer errors.
    #[derive(Debug)]
    pub enum Error {
        /// The writer is closed.
        Closed {
            description("the writer is closed")
        }
        /// The attributes of the point format and point do not match.
        PointAttributes(format: Format, point: Point) {
            description("the attributes of the point format and point do not match")
            display("the attributes of point format {:?} does not match point {:?}", format, point)
        }
        /// Wrapper around `std::io::Error`.
        Io(err: std::io::Error) {
            from()
            cause(err)
            description(err.description())
            display("io error: {}", err)
        }
    }
}

pub(crate) fn write_point_to<W: Write>(
    mut dst: &mut W,
    point: Point,
    header: &Header,
) -> Result<()> {
    point
        .into_raw(header.transforms())
        .and_then(|raw_point| raw_point.write_to(&mut dst, header.point_format()))?;
    Ok(())
}

/// Trait that defines a PointWriter, s
pub(crate) trait PointWriter<W: Write>: Debug {
    fn write_next(&mut self, point: Point) -> Result<()>;
    //https://users.rust-lang.org/t/is-there-a-way-to-move-a-trait-object/707
    fn into_inner(self: Box<Self>) -> W;
    fn get_mut(&mut self) -> &mut W;
    fn header(&self) -> &Header;
    // Needed because the compressed point writer needs to be told when its done encoding data
    fn done(&mut self) -> Result<()>;
}

/// This struct is used to be able to get the inner stream of the writer when
/// calling `into_inner`
#[derive(Debug)]
struct UnreachablePointWriter {}

impl<W: Write> PointWriter<W> for UnreachablePointWriter {
    fn write_next(&mut self, _point: Point) -> Result<()> {
        unreachable!()
    }

    fn into_inner(self: Box<Self>) -> W {
        unreachable!()
    }

    fn get_mut(&mut self) -> &mut W {
        unreachable!()
    }

    fn header(&self) -> &Header {
        unreachable!()
    }

    fn done(&mut self) -> Result<()> {
        unreachable!()
    }
}

#[derive(Debug)]
struct UncompressedPointWriter<W: Write + Debug> {
    dest: W,
    header: Header,
}

impl<W: Write + Debug> PointWriter<W> for UncompressedPointWriter<W> {
    fn write_next(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        write_point_to(&mut self.dest, point, &self.header)?;
        Ok(())
    }

    fn into_inner(self: Box<Self>) -> W {
        self.dest
    }

    fn get_mut(&mut self) -> &mut W {
        &mut self.dest
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn done(&mut self) -> Result<()> {
        Ok(())
    }
}

pub(crate) fn write_header_and_vlrs_to<W: Write>(mut dest: &mut W, header: &Header) -> Result<()> {
    header
        .clone()
        .into_raw()
        .and_then(|raw_header| raw_header.write_to(&mut dest))?;
    for vlr in header.vlrs() {
        (*vlr)
            .clone()
            .into_raw(false)
            .and_then(|raw_vlr| raw_vlr.write_to(&mut dest))?;
    }
    if !header.vlr_padding().is_empty() {
        dest.write_all(&header.vlr_padding())?;
    }
    Ok(())
}

/// Writes LAS data.
///
/// See StdWriter for a concrete implementation.
pub trait Writer {
    /// Returns a reference to this writer's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{StdWriter, Writer};
    /// let writer = StdWriter::default();
    /// let header = writer.header();
    /// ```
    fn header(&self) -> &Header;

    /// Writes a point
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::{StdWriter, Writer};
    ///
    /// let mut writer = StdWriter::default();
    /// writer.write(Default::default()).unwrap();
    /// ```
    fn write(&mut self, point: Point) -> Result<()>;
}

/// Writes LAS data.
///
/// The LAS header needs to be re-written when the writer closes. For convenience, this is done via
/// the `Drop` implementation of the writer. One consequence is that if the header re-write fails
/// during the drop, a panic will result. If you want to check for errors instead of panicing, use
/// `close` explicitly.
///
/// ```
/// use std::io::Cursor;
/// use las::StdWriter;
/// {
///     let mut writer = StdWriter::default();
///     writer.close().unwrap();
/// } // <- `close` is not called
/// ```
#[derive(Debug)]
pub struct StdWriter<W: 'static + Write + Seek + Debug> {
    closed: bool,
    start: u64,
    point_writer: Box<dyn PointWriter<W>>,
}

impl<W: 'static + Write + Seek + Debug> StdWriter<W> {
    /// Creates a new writer.
    ///
    /// The header that is passed in will have various fields zero'd, e.g. bounds, number of
    /// points, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::StdWriter;
    /// let writer = StdWriter::new(Cursor::new(Vec::new()), Default::default());
    /// ```
    pub fn new(mut dest: W, mut header: Header) -> Result<Self> {
        let start = dest.seek(SeekFrom::Current(0))?;
        header.clear();

        #[cfg(feature = "laz")]
        {
            if header.point_format().is_compressed {
                Ok(Self {
                    closed: false,
                    start,
                    point_writer: Box::new(CompressedPointWriter::new(dest, header)?),
                })
            } else {
                write_header_and_vlrs_to(&mut dest, &header)?;
                Ok(Self {
                    closed: false,
                    start,
                    point_writer: Box::new(UncompressedPointWriter { dest, header }),
                })
            }
        }
        #[cfg(not(feature = "laz"))]
        {
            write_header_and_vlrs_to(&mut dest, &header)?;
            Ok(StdWriter {
                closed: false,
                start,
                point_writer: Box::new(UncompressedPointWriter { dest, header }),
            })
        }
    }

    /// Close this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::StdWriter;
    /// let mut writer = StdWriter::default();
    /// writer.close().unwrap();
    /// assert!(writer.close().is_err());
    /// ```
    pub fn close(&mut self) -> Result<()> {
        if self.closed {
            return Err(Error::Closed.into());
        }

        self.point_writer.done()?;

        let point_padding = self.header().point_padding().clone();
        self.point_writer.get_mut().write_all(&point_padding)?;
        let raw_evlrs: Vec<Result<crate::raw::Vlr>> = {
            self.point_writer
                .header()
                .evlrs()
                .iter()
                .map(|evlr| evlr.clone().into_raw(true))
                .collect()
        };

        for raw_evlr in raw_evlrs {
            raw_evlr?.write_to(&mut self.point_writer.get_mut())?;
        }

        self.point_writer
            .get_mut()
            .seek(SeekFrom::Start(self.start))?;
        self.header()
            .clone()
            .into_raw()
            .and_then(|raw_header| raw_header.write_to(&mut self.point_writer.get_mut()))?;
        self.point_writer
            .get_mut()
            .seek(SeekFrom::Start(self.start))?;
        self.closed = true;
        Ok(())
    }
}

impl<W: 'static + Write + Seek + Debug> Writer for StdWriter<W> {
    /// Returns the header.
    fn header(&self) -> &Header {
        &self.point_writer.header()
    }

    /// Writes a point.
    fn write(&mut self, point: Point) -> Result<()> {
        if self.closed {
            return Err(Error::Closed.into());
        }
        if !point.matches(self.header().point_format()) {
            return Err(Error::PointAttributes(*self.header().point_format(), point).into());
        }
        self.point_writer.write_next(point)
    }
}

impl<W: 'static + Write + Seek + Debug> StdWriter<W> {
    /// Closes this writer and returns its inner `Write`, seeked to the beginning of the las data.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::StdWriter;
    /// let writer = StdWriter::default();
    /// let cursor = writer.into_inner().unwrap();
    /// ```
    pub fn into_inner(mut self) -> Result<W> {
        if !self.closed {
            self.close()?;
        }

        // since Writer implements Drop, the stream cannot be moved
        // to work around this, we replace the current point writer with a point writer
        // that will panic on any of the PintWriter fn call, basically meaning
        // that any call to a Writer function after close can potentially cause a panic
        // if the method does no checks for self.closed before, which should not be
        // a problem as this function moves the writer, meaning the user won't have
        // access to it anymore
        let point_writer =
            std::mem::replace(&mut self.point_writer, Box::new(UnreachablePointWriter {}));
        let mut inner = point_writer.into_inner();
        inner.seek(SeekFrom::Start(self.start))?;
        Ok(inner)
    }
}

impl StdWriter<BufWriter<File>> {
    /// Creates a new writer for a path.
    ///
    /// If the "laz" feature is enabled, guesses from the extension if the
    /// data will be written compressed
    ///
    /// # Examples
    ///
    /// ```
    /// use las::StdWriter;
    /// let writer = StdWriter::from_path("/dev/null", Default::default());
    /// ```
    pub fn from_path<P: AsRef<Path>>(
        path: P,
        mut header: Header,
    ) -> Result<StdWriter<BufWriter<File>>> {
        let compress = if cfg!(feature = "laz") {
            match path.as_ref().extension() {
                Some(ext) => match &ext.to_str() {
                    Some(ext_str) => {
                        if &ext_str.to_lowercase() == "laz" {
                            true
                        } else {
                            false
                        }
                    }
                    None => false,
                },
                None => false,
            }
        } else {
            false
        };

        header.point_format_mut().is_compressed = compress;
        File::create(path)
            .map_err(::Error::from)
            .and_then(|file| StdWriter::new(BufWriter::new(file), header))
    }
}

impl Default for StdWriter<Cursor<Vec<u8>>> {
    fn default() -> StdWriter<Cursor<Vec<u8>>> {
        StdWriter::new(Cursor::new(Vec::new()), Header::default()).unwrap()
    }
}

impl<W: 'static + Seek + Write + Debug> Drop for StdWriter<W> {
    fn drop(&mut self) {
        if !self.closed {
            self.close().expect("Error when dropping the writer");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use header::Builder;
    use point::Format;
    use Version;

    use super::*;

    fn writer(format: Format, version: Version) -> StdWriter<Cursor<Vec<u8>>> {
        let mut builder = Builder::default();
        builder.point_format = format;
        builder.version = version;
        StdWriter::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap()
    }

    #[test]
    fn already_closed() {
        let mut writer = StdWriter::default();
        writer.close().unwrap();
        assert!(writer.close().is_err());
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_extra_bytes() {
        let format = Format {
            extra_bytes: 1,
            ..Default::default()
        };
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_gps_time() {
        let format = Format::new(1).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_color() {
        let format = Format::new(2).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_nir() {
        let format = Format::new(8).unwrap();
        let mut writer = writer(format, Version::new(1, 4));
        let point = Point {
            gps_time: Some(0.),
            color: Some(Default::default()),
            ..Default::default()
        };
        assert!(writer.write(point).is_err());
    }

    #[test]
    fn missing_waveform() {
        let format = Format::new(4).unwrap();
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn write_not_at_start() {
        use byteorder::WriteBytesExt;
        use {Reader, StdReader};

        let mut cursor = Cursor::new(Vec::new());
        cursor.write_u8(42).unwrap();
        let mut writer = StdWriter::new(cursor, Default::default()).unwrap();
        let point = Point::default();
        writer.write(point.clone()).unwrap();
        let mut reader = StdReader::new(writer.into_inner().unwrap()).unwrap();
        assert_eq!(point, reader.read().unwrap().unwrap());
    }
}
