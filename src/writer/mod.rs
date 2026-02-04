//! Write las points.
//!
//! A `Writer` uses a `Header` for its configuration:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Header};
//! let mut header = Header::from((1, 4));
//! let writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! The set of optional fields on the point format and the points must match exactly:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Builder, Write, Writer, Point};
//! use las::point::Format;
//! use las::Color;
//!
//! let mut builder = Builder::default();
//! builder.point_format = Format::new(1).unwrap();
//! let mut writer = Writer::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap();
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

mod las;
#[cfg(feature = "laz")]
mod laz;

#[cfg(feature = "laz")]
use crate::LazParallelism;
use crate::{Error, Header, Point, Result};
use std::{
    fmt::Debug,
    fs::File,
    io::{BufWriter, Cursor, Seek, SeekFrom},
    path::Path,
};

trait WritePoint<W: std::io::Write>: Send {
    fn write_point(&mut self, point: Point) -> Result<()>;
    fn write_points(&mut self, points: &[Point]) -> Result<()> {
        for point in points.iter().cloned() {
            self.write_point(point)?;
        }
        Ok(())
    }
    //https://users.rust-lang.org/t/is-there-a-way-to-move-a-trait-object/707
    fn into_inner(self: Box<Self>) -> W;
    fn get_mut(&mut self) -> &mut W;
    fn header(&self) -> &Header;
    fn header_mut(&mut self) -> &mut Header;
    fn done(&mut self) -> Result<()>;
}

struct ClosedPointWriter;

impl<W: std::io::Write> WritePoint<W> for ClosedPointWriter {
    fn write_point(&mut self, _point: Point) -> Result<()> {
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
    fn header_mut(&mut self) -> &mut Header {
        unreachable!()
    }
    fn done(&mut self) -> Result<()> {
        unreachable!()
    }
}

/// Writes LAS data.
///
/// See StdWriter for a concrete implementation.
#[deprecated(
    since = "0.9.0",
    note = "This interface has been refactored so that importing Write is no longer required"
)]
pub trait Write {
    /// Returns a reference to this writer's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::default();
    /// let header = writer.header();
    /// ```
    fn header(&self) -> &Header;

    /// Writes a point
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::Writer;
    ///
    /// let mut writer = Writer::default();
    /// writer.write(Default::default()).unwrap();
    /// ```
    fn write(&mut self, point: Point) -> Result<()>;
}

/// Options for Writer
///
/// Currently, the only option is the selection of LAZ parallelism via [LazParallelism].
/// This option requires the `laz` feature to be enabled (and to use parallelism, the `laz-parallel`
/// feature must also be enabled).
/// Using parallel writing will speedup compression when writing points in batch, at the cost
/// of slightly more memory consumption.
///
/// By default, if the `laz-parallel` feature is enabled, parallelism will be the default choice
#[derive(Debug, Clone, Copy)]
pub struct WriterOptions {
    #[cfg(feature = "laz")]
    laz_parallelism: LazParallelism,
}

impl WriterOptions {
    /// Change the laz parallelism option
    #[cfg(feature = "laz")]
    pub fn with_laz_parallelism(mut self, laz_parallelism: LazParallelism) -> Self {
        self.laz_parallelism = laz_parallelism;
        self
    }
}

impl Default for WriterOptions {
    fn default() -> Self {
        #[cfg(feature = "laz-parallel")]
        {
            Self {
                laz_parallelism: LazParallelism::Yes,
            }
        }
        #[cfg(all(feature = "laz", not(feature = "laz-parallel")))]
        {
            Self {
                laz_parallelism: LazParallelism::No,
            }
        }

        #[cfg(not(feature = "laz"))]
        {
            Self {}
        }
    }
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
/// use las::Writer;
/// {
///     let mut writer = Writer::default();
///     writer.close().unwrap();
/// } // <- `close` is not called
/// ```
#[allow(missing_debug_implementations)]
pub struct Writer<W: 'static + std::io::Write + Seek + Send + Sync> {
    closed: bool,
    start: u64,
    point_writer: Box<dyn WritePoint<W> + Send>,
}

impl<W: 'static + std::io::Write + Seek + Send + Sync> Writer<W> {
    /// Creates a new writer.
    ///
    /// The header that is passed in will have various fields zero'd, e.g. bounds, number of
    /// points, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::Writer;
    /// let writer = Writer::new(Cursor::new(Vec::new()), Default::default());
    /// ```
    pub fn new(write: W, header: Header) -> Result<Writer<W>> {
        Self::with_options(write, header, WriterOptions::default())
    }

    /// Creates a new writer with custom options
    ///
    /// The header that is passed in will have various fields zero'd, e.g. bounds, number of
    /// points, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "laz-parallel")]
    /// # fn main() {
    /// use std::io::Cursor;
    /// use las::{Writer, WriterOptions, LazParallelism};
    /// let options = WriterOptions::default()
    ///     .with_laz_parallelism(LazParallelism::Yes);
    /// let writer = Writer::with_options(Cursor::new(Vec::new()), Default::default(), options);
    /// # }
    /// # #[cfg(not(feature = "laz-parallel"))]
    /// # fn main() {}
    /// ```
    pub fn with_options(mut write: W, mut header: Header, options: WriterOptions) -> Result<Self> {
        let start = write.stream_position()?;
        header.clear();
        if header.point_format().is_compressed {
            #[cfg(feature = "laz")]
            {
                header.add_laz_vlr()?;
                header.write_to(&mut write)?;

                let point_writer: Box<dyn WritePoint<W>> = match options.laz_parallelism {
                    #[cfg(feature = "laz-parallel")]
                    LazParallelism::Yes => {
                        laz::PointWriter::new_parallel(write, header).map(Box::new)?
                    }
                    LazParallelism::No => laz::PointWriter::new(write, header).map(Box::new)?,
                };

                Ok(Writer {
                    closed: false,
                    start,
                    point_writer,
                })
            }
            #[cfg(not(feature = "laz"))]
            {
                Err(Error::LaszipNotEnabled)
            }
        } else {
            // Silence unused variable warning as the only option is related to laz
            let _ = options;
            header.write_to(&mut write)?;
            Ok(Writer {
                closed: false,
                start,
                point_writer: Box::new(las::PointWriter::new(write, header)),
            })
        }
    }

    /// Close this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::Writer;
    /// let mut writer = Writer::default();
    /// writer.close().unwrap();
    /// assert!(writer.close().is_err());
    /// ```
    pub fn close(&mut self) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }

        self.point_writer.done()?;

        let point_padding = self.header().point_padding().clone();
        self.point_writer.get_mut().write_all(&point_padding)?;

        let start_of_first_evlr = self.point_writer.get_mut().stream_position()?;
        self.point_writer
            .header_mut()
            .set_start_of_first_evlr(start_of_first_evlr);
        let raw_evlrs: Vec<Result<crate::raw::Vlr>> = {
            self.point_writer
                .header()
                .evlrs()
                .iter()
                .map(|evlr| evlr.clone().into_raw(true))
                .collect()
        };

        for raw_evlr in raw_evlrs {
            raw_evlr?.write_to(self.point_writer.get_mut())?;
        }

        let _ = self
            .point_writer
            .get_mut()
            .seek(SeekFrom::Start(self.start))?;
        self.header()
            .clone()
            .into_raw()
            .and_then(|raw_header| raw_header.write_to(self.point_writer.get_mut()))?;
        let _ = self
            .point_writer
            .get_mut()
            .seek(SeekFrom::Start(self.start))?;
        self.closed = true;
        Ok(())
    }

    /// Returns a reference to this writer's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    ///
    /// let writer = Writer::default();
    /// let header = writer.header();
    /// ```
    pub fn header(&self) -> &Header {
        self.point_writer.header()
    }

    /// Writes a point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    ///
    /// let mut writer = Writer::default();
    /// writer.write_point(Default::default()).unwrap();
    /// ```
    pub fn write_point(&mut self, point: Point) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }
        if !point.matches(self.header().point_format()) {
            return Err(Error::PointAttributesDoNotMatch(
                *self.header().point_format(),
            ));
        }
        self.point_writer.write_point(point)
    }

    /// Writes all the points
    pub fn write_points(&mut self, points: &[Point]) -> Result<()> {
        if self.closed {
            return Err(Error::ClosedWriter);
        }

        if points
            .iter()
            .any(|point| !point.matches(self.header().point_format()))
        {
            return Err(Error::PointAttributesDoNotMatch(
                *self.header().point_format(),
            ));
        }

        self.point_writer.write_points(points)
    }

    /// Writes a point.
    #[deprecated(since = "0.9.0", note = "Use write_point() instead")]
    pub fn write(&mut self, point: Point) -> Result<()> {
        self.write_point(point)
    }

    /// Closes this writer and returns its inner `Write`, seeked to the beginning of the las data.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::default();
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
        let point_writer = std::mem::replace(&mut self.point_writer, Box::new(ClosedPointWriter));
        let mut inner = point_writer.into_inner();
        let _ = inner.seek(SeekFrom::Start(self.start))?;
        Ok(inner)
    }
}

#[allow(deprecated)]
impl<W: 'static + std::io::Write + Seek + Debug + Send + Sync> Write for Writer<W> {
    fn header(&self) -> &Header {
        self.header()
    }

    fn write(&mut self, point: Point) -> Result<()> {
        self.write(point)
    }
}

impl Writer<BufWriter<File>> {
    /// Creates a new writer for a path.
    ///
    /// If the "laz" feature is enabled, guesses from the extension if the
    /// data will be written compressed
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Writer;
    /// let writer = Writer::from_path("/dev/null", Default::default());
    /// ```
    pub fn from_path<P: AsRef<Path>>(
        path: P,
        mut header: Header,
    ) -> Result<Writer<BufWriter<File>>> {
        let compress = if cfg!(feature = "laz") {
            match path.as_ref().extension() {
                Some(ext) => match &ext.to_str() {
                    Some(ext_str) => &ext_str.to_lowercase() == "laz",
                    None => false,
                },
                None => false,
            }
        } else {
            false
        };

        header.point_format_mut().is_compressed = compress;
        File::create(path)
            .map_err(Error::from)
            .and_then(|file| Writer::new(BufWriter::new(file), header))
    }
}

impl Default for Writer<Cursor<Vec<u8>>> {
    fn default() -> Writer<Cursor<Vec<u8>>> {
        Writer::new(Cursor::new(Vec::new()), Header::default()).unwrap()
    }
}

impl<W: 'static + Seek + std::io::Write + Send + Sync> Drop for Writer<W> {
    fn drop(&mut self) {
        if !self.closed {
            self.close().expect("Error when dropping the writer");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{header::Builder, point::Format, Version};
    use std::io::Cursor;

    fn writer(format: Format, version: Version) -> Writer<Cursor<Vec<u8>>> {
        let mut builder = Builder::default();
        builder.point_format = format;
        builder.version = version;
        Writer::new(Cursor::new(Vec::new()), builder.into_header().unwrap()).unwrap()
    }

    #[test]
    fn already_closed() {
        let mut writer = Writer::default();
        writer.close().unwrap();
        assert!(writer.close().is_err());
        assert!(writer.write_point(Default::default()).is_err());
    }

    #[test]
    fn missing_extra_bytes() {
        let format = Format {
            extra_bytes: 1,
            ..Default::default()
        };
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write_point(Default::default()).is_err());
    }

    #[test]
    fn missing_gps_time() {
        let format = Format::new(1).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write_point(Default::default()).is_err());
    }

    #[test]
    fn missing_color() {
        let format = Format::new(2).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write_point(Default::default()).is_err());
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
        assert!(writer.write_point(point).is_err());
    }

    #[test]
    fn missing_waveform() {
        let format = Format::new(4).unwrap();
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write_point(Default::default()).is_err());
    }

    #[test]
    fn write_not_at_start() {
        use crate::Reader;
        use byteorder::WriteBytesExt;

        let mut cursor = Cursor::new(Vec::new());
        cursor.write_u8(42).unwrap();
        let mut writer = Writer::new(cursor, Default::default()).unwrap();
        let point = Point::default();
        writer.write_point(point.clone()).unwrap();
        let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
        assert_eq!(point, reader.read_point().unwrap().unwrap());
    }
}
