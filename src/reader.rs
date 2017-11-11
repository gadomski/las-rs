//! Read las points.
//!
//! If you're reading any significant number of points, you'll want to make sure you're using a
//! `BufRead` instead of just a `Read`:
//!
//! ```
//! use std::fs::File;
//! use std::io::BufReader;
//! use las::Reader;
//!
//! let read = BufReader::new(File::open("tests/data/autzen.las").unwrap());
//! let reader = Reader::new(read).unwrap();
//! ```
//!
//! `Reader::from_path` does this for you:
//!
//! ```
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! ```
//!
//! For now, compressed files are not supported:
//!
//! ```
//! use las::Reader;
//! assert!(Reader::from_path("tests/data/autzen.laz").is_err());
//! ```
//!
//! Use `Reader::read` to read one point, and `Reader::points` to get an iterator over
//! `Result<Point>`:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let first_point = reader.read().unwrap().unwrap();
//! let the_rest = reader.points().map(|r| r.unwrap()).collect::<Vec<_>>();
//! ```

use {Builder, Header, Point, Result, Vlr, raw};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

quick_error! {
    /// Error while reading.
    #[derive(Clone, Copy, Debug)]
    pub enum Error {
        /// The offset to the point data was too small.
        OffsetToPointDataTooSmall(offset: u32) {
            description("offset to point data too small")
            display("offset to point data too small: {}", offset)
        }
        /// The offset to the start of the evlrs is too small.
        OffsetToEvlrsTooSmall(offset: u64) {
            description("offset the evlrs is too small")
            display("offset to the evlrs is too small: {}", offset)
        }
    }
}

/// Reads LAS data.
#[derive(Debug)]
pub struct Reader<R: Read + Seek> {
    header: Header,
    read: R,
    number_of_points: u64,
    number_read: u64,
}

impl<R: Read + Seek> Reader<R> {
    /// Creates a new reader.
    ///
    /// This does *not* wrap the `Read` in a `BufRead`, so if you're concered about performance you
    /// should do that wrapping yourself (or use `from_path`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::BufReader;
    /// use std::fs::File;
    /// # use las::Reader;
    /// let file = File::open("tests/data/autzen.las").unwrap();
    /// let reader = Reader::new(BufReader::new(file)).unwrap();
    /// ```
    pub fn new(mut read: R) -> Result<Reader<R>> {
        let raw_header = raw::Header::read_from(&mut read)?;
        if raw_header.point_format.is_compressed {
            return Err(::Error::Laszip);
        }
        let mut position = u64::from(raw_header.header_size);
        let number_of_variable_length_records = raw_header.number_of_variable_length_records;
        let offset_to_point_data = u64::from(raw_header.offset_to_point_data);
        let offset_to_end_of_points = raw_header.offset_to_end_of_points();
        let evlr = raw_header.evlr;

        let mut builder = Builder::new(raw_header)?;
        for _ in 0..number_of_variable_length_records {
            let vlr = raw::Vlr::read_from(&mut read, false).and_then(Vlr::new)?;
            position += vlr.len() as u64;
            builder.vlrs.push(vlr);
        }
        if position > offset_to_point_data {
            return Err(
                Error::OffsetToPointDataTooSmall(offset_to_point_data as u32).into(),
            );
        } else if position < offset_to_point_data {
            read.by_ref()
                .take(offset_to_point_data - position)
                .read_to_end(&mut builder.vlr_padding)?;
        }

        read.seek(SeekFrom::Start(offset_to_end_of_points))?;
        if let Some(evlr) = evlr {
            if evlr.start_of_first_evlr < offset_to_end_of_points {
                return Err(
                    Error::OffsetToEvlrsTooSmall(evlr.start_of_first_evlr).into(),
                );
            } else if evlr.start_of_first_evlr > offset_to_end_of_points {
                let n = evlr.start_of_first_evlr - offset_to_end_of_points;
                read.by_ref().take(n).read_to_end(
                    &mut builder.point_padding,
                )?;
            }
            builder.vlrs.push(
                raw::Vlr::read_from(&mut read, true).and_then(
                    Vlr::new,
                )?,
            );
        }

        read.seek(SeekFrom::Start(offset_to_point_data))?;
        let header = builder.into_header()?;

        Ok(Reader {
            number_of_points: header.number_of_points(),
            header: header,
            read: read,
            number_read: 0,
        })
    }

    /// Returns a reference to this reader's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let header = reader.header();
    /// ```
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Reads a point.
    ///
    /// Returns `Ok(None)` if we have already read the last point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let point = reader.read().unwrap().unwrap();
    /// ```
    pub fn read(&mut self) -> Result<Option<Point>> {
        if self.number_read >= self.number_of_points {
            Ok(None)
        } else {
            let point = raw::Point::read_from(&mut self.read, self.header.point_format())
                .map(|raw_point| {
                    Some(Point::new(raw_point, self.header.transforms()))
                });
            self.number_read += 1;
            point
        }
    }

    /// Returns an iterator over this reader's points.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = reader.points().collect::<Result<Vec<_>, _>>().unwrap();
    /// ```
    pub fn points(&mut self) -> Points<R> {
        Points { reader: self }
    }
}

impl Reader<BufReader<File>> {
    /// Creates a new reader from a path.
    ///
    /// The underlying `File` is wrapped in a `BufReader` for performance reasons.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        File::open(path).map_err(::Error::from).and_then(|file| {
            Reader::new(BufReader::new(file))
        })
    }
}

/// An iterator over of the points in a `Reader`.
///
/// This struct is generally created by calling `points()` on `Reader`.
#[derive(Debug)]
pub struct Points<'a, R: 'a + Read + Seek> {
    reader: &'a mut Reader<R>,
}

impl<'a, R: Read + Seek> Iterator for Points<'a, R> {
    type Item = Result<Point>;
    fn next(&mut self) -> Option<Result<Point>> {
        match self.reader.read() {
            Ok(None) => None,
            Ok(Some(point)) => Some(Ok(point)),
            Err(err) => Some(Err(err)),
        }
    }
}
