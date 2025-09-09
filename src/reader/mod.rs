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
//! Use `Reader::read` to read one point, and `Reader::points` to get an iterator over
//! `Result<Point>`:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let first_point = reader.read().unwrap().unwrap();
//! let the_rest = reader.points().map(|r| r.unwrap()).collect::<Vec<_>>();
//! ```
//!
//! # Compression
//!
//! [lazip](https://laszip.org/) is supported by enabling the `laz` feature in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! las = { version = "*", features = ["laz"] }
//! ```
//!
//! Then:
//!
//! ```
//! # #[cfg(feature = "laz")]
//! # {
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.laz").unwrap();
//! # }
//! ```
//!

mod las;
#[cfg(feature = "laz")]
mod laz;

use crate::{Error, Header, Point, Result};
use std::{
    fs::File,
    io::{BufReader, Seek},
    path::Path,
};

trait ReadPoints {
    fn read_point(&mut self) -> Result<Option<Point>>;
    fn read_points(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64>;
    fn seek(&mut self, index: u64) -> Result<()>;
    fn header(&self) -> &Header;
}

/// An iterator over of the points in a `Reader`.
///
/// This struct is generally created by calling `points()` on `Reader`.
#[allow(missing_debug_implementations)]
pub struct PointIterator<'a> {
    point_reader: &'a mut dyn ReadPoints,
}

impl Iterator for PointIterator<'_> {
    type Item = Result<Point>;

    fn next(&mut self) -> Option<Self::Item> {
        self.point_reader.read_point().transpose()
    }
}

/// A trait for objects which read LAS data.
#[deprecated(
    since = "0.9.0",
    note = "This interface has been refactored so that importing `Read` is no longer required"
)]
pub trait Read {
    /// Returns a reference to this reader's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let header = reader.header();
    /// ```
    fn header(&self) -> &Header;

    /// Reads a point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let point = reader.read().unwrap().unwrap();
    /// ```
    fn read(&mut self) -> Option<Result<Point>>;

    /// Reads n points.
    fn read_n(&mut self, n: u64) -> Result<Vec<Point>>;

    /// Reads n points into the vec
    fn read_n_into(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64>;

    /// Reads all points left into the vec
    fn read_all_points(&mut self, points: &mut Vec<Point>) -> Result<u64>;

    /// Seeks to the given point number, zero-indexed.
    ///
    /// Note that seeking on compressed (LAZ) data can be expensive as the reader
    /// will have to seek to the closest chunk start and decompress all points up until
    /// the point seeked to.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// reader.seek(1).unwrap(); // <- seeks to the second point
    /// let the_second_point = reader.read().unwrap().unwrap();
    /// ```
    fn seek(&mut self, position: u64) -> Result<()>;

    /// Returns an iterator over this reader's points.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = reader.points().collect::<Result<Vec<_>, _>>().unwrap();
    /// ```
    fn points(&mut self) -> PointIterator<'_>;
}

/// Choice of laz parallelism.
#[cfg(feature = "laz")]
#[derive(Debug, Clone, Copy)]
pub enum LazParallelism {
    #[cfg(feature = "laz-parallel")]
    /// Use parallel laz decompression / compression
    Yes,
    /// Do not use parallel laz decompression / compression
    No,
}

/// Options for Reader.
///
/// Currently, the only option is the selection of LAZ parallelism via [LazParallelism].
/// This option requires the `laz` feature to be enabled (and to use parallelism, the `laz-parallel`
/// feature must also be enabled)
/// By default, if the `laz-parallel` feature is enabled, parallelism will be the default choice
#[derive(Debug, Clone, Copy)]
pub struct ReaderOptions {
    #[cfg(feature = "laz")]
    laz_parallelism: LazParallelism,
}

impl ReaderOptions {
    /// Change the laz parallelism option
    #[cfg(feature = "laz")]
    pub fn with_laz_parallelism(mut self, laz_parallelism: LazParallelism) -> Self {
        self.laz_parallelism = laz_parallelism;
        self
    }
}

impl Default for ReaderOptions {
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

/// Reads LAS data.
#[allow(missing_debug_implementations)]
pub struct Reader {
    point_reader: Box<dyn ReadPoints>,
}

impl Reader {
    /// Creates a new reader with default options.
    ///
    /// This does *not* wrap the `Read` in a `BufRead`, so if you're concerned
    /// about performance you should do that wrapping yourself (or use
    /// `from_path`).
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
    pub fn new<R: std::io::Read + Seek + Send + Sync + 'static>(read: R) -> Result<Reader> {
        Self::with_options(read, ReaderOptions::default())
    }

    /// Creates a new reader with custom options.
    ///
    /// This does *not* wrap the `Read` in a `BufRead`, so if you're concerned
    /// about performance you should do that wrapping yourself (or use
    /// `from_path`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::BufReader;
    /// use std::fs::File;
    /// # use las::{Reader, ReaderOptions};
    /// let file = File::open("tests/data/autzen.las").unwrap();
    /// let reader = Reader::with_options(BufReader::new(file), ReaderOptions::default()).unwrap();
    /// ```
    pub fn with_options<R: std::io::Read + Seek + Send + Sync + 'static>(
        mut read: R,
        options: ReaderOptions,
    ) -> Result<Reader> {
        let header = Header::new(&mut read)?;
        if header.point_format().is_compressed {
            #[cfg(feature = "laz")]
            {
                let point_reader: Box<dyn ReadPoints> = match options.laz_parallelism {
                    #[cfg(feature = "laz-parallel")]
                    LazParallelism::Yes => {
                        laz::PointReader::new_parallel(read, header).map(Box::new)?
                    }
                    LazParallelism::No => laz::PointReader::new(read, header).map(Box::new)?,
                };

                Ok(Reader { point_reader })
            }
            #[cfg(not(feature = "laz"))]
            {
                Err(Error::LaszipNotEnabled)
            }
        } else {
            // Silence unused variable warning as the only option is related to laz
            let _ = options;
            Ok(Reader {
                point_reader: Box::new(las::PointReader::new(read, header)?),
            })
        }
    }

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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader> {
        File::open(path)
            .map_err(Error::from)
            .and_then(|file| Reader::new(BufReader::new(file)))
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
        self.point_reader.header()
    }

    /// Reads a point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let point = reader.read_point().unwrap().unwrap();
    /// ```
    pub fn read_point(&mut self) -> Result<Option<Point>> {
        self.point_reader.read_point()
    }

    /// Reads `n` points into a vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = reader.read_points(10).unwrap();
    /// assert_eq!(points.len(), 10);
    /// ```
    pub fn read_points(&mut self, n: u64) -> Result<Vec<Point>> {
        let mut points = Vec::with_capacity(n.try_into()?);
        let _ = self.point_reader.read_points(n, &mut points)?;
        Ok(points)
    }

    /// Reads `n` points into a provided vector, returning the number of points read.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let mut points = Vec::new();
    /// let count = reader.read_points_into(10, &mut points).unwrap();
    /// ```
    pub fn read_points_into(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64> {
        self.point_reader.read_points(n, points)
    }

    /// Reads a point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let point = reader.read().unwrap().unwrap();
    /// ```
    #[deprecated(
        since = "0.9.0",
        note = "Use read_point() instead, which returns a Result<Option<Point>>"
    )]
    pub fn read(&mut self) -> Option<Result<Point>> {
        self.point_reader.read_point().transpose()
    }

    /// Reads n points into a vector.
    #[deprecated(since = "0.9.0", note = "Use read_points() instead")]
    pub fn read_n(&mut self, n: u64) -> Result<Vec<Point>> {
        self.read_points(n)
    }

    /// Reads n points into the vec
    #[deprecated(since = "0.9.0", note = "Use read_points_into() instead")]
    pub fn read_n_into(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64> {
        self.read_points_into(n, points)
    }

    /// Reads all points into a vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let mut points = Vec::new();
    /// let count = reader.read_all_points(&mut points).unwrap();
    /// assert_eq!(points.len(), count.try_into().unwrap());
    /// ```
    pub fn read_all_points_into(&mut self, points: &mut Vec<Point>) -> Result<u64> {
        let point_count = self.point_reader.header().number_of_points();
        self.point_reader.read_points(point_count, points)
    }

    /// Reads all points into a vector.
    #[deprecated(since = "0.9.0", note = "Use read_all_points_into() instead")]
    pub fn read_all_points(&mut self, points: &mut Vec<Point>) -> Result<u64> {
        self.read_all_points_into(points)
    }

    /// Seeks to the given point number, zero-indexed.
    ///
    /// Note that seeking on compressed (LAZ) data can be expensive as the reader
    /// will have to seek to the closest chunk start and decompress all points up until
    /// the point seeked to.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// reader.seek(1).unwrap(); // <- seeks to the second point
    /// let the_second_point = reader.read().unwrap().unwrap();
    /// ```
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.point_reader.seek(position)
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
    pub fn points(&mut self) -> PointIterator<'_> {
        PointIterator {
            point_reader: &mut *self.point_reader,
        }
    }
}

#[allow(deprecated)]
impl Read for Reader {
    /// Returns a reference to this reader's header.
    fn header(&self) -> &Header {
        self.header()
    }

    /// Reads a point.
    fn read(&mut self) -> Option<Result<Point>> {
        self.read()
    }

    fn read_n(&mut self, n: u64) -> Result<Vec<Point>> {
        self.read_n(n)
    }

    fn read_n_into(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64> {
        self.read_n_into(n, points)
    }

    fn read_all_points(&mut self, points: &mut Vec<Point>) -> Result<u64> {
        self.read_all_points(points)
    }

    /// Seeks to the given point number, zero-indexed.
    fn seek(&mut self, position: u64) -> Result<()> {
        self.seek(position)
    }

    /// Returns an iterator over this reader's points.
    fn points(&mut self) -> PointIterator<'_> {
        self.points()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Writer;

    #[test]
    fn seek() {
        let mut writer = Writer::default();
        writer.write_point(Default::default()).unwrap();
        let point = Point {
            x: 1.,
            y: 2.,
            z: 3.,
            ..Default::default()
        };
        writer.write_point(point.clone()).unwrap();
        let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
        reader.seek(1).unwrap();
        assert_eq!(point, reader.read_point().unwrap().unwrap());
        assert!(reader.read_point().unwrap().is_none());
    }
}
