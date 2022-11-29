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
//! Ccompressed files are supported when using the feature "laz":
//!
//! ```
//! use las::Reader;
//! if cfg!(feature = "laz") {
//!  assert!(Reader::from_path("tests/data/autzen.laz").is_ok());
//! } else {
//!  assert!(Reader::from_path("tests/data/autzen.laz").is_err());
//! }
//!
//! ```
//!
//! Use `Reader::read` to read one point, and `Reader::points` to get an iterator over
//! `Result<Point>`:
//!
//! ```
//! use las::{Read, Reader};
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let first_point = reader.read().unwrap().unwrap();
//! let the_rest = reader.points().map(|r| r.unwrap()).collect::<Vec<_>>();
//! ```

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

#[cfg(feature = "laz")]
use crate::compression::CompressedPointReader;

use crate::{raw, Builder, Header, Point, Result, Vlr};
use std::{cmp::Ordering, fmt::Debug};
use thiserror::Error;

/// Error while reading.
#[derive(Error, Clone, Copy, Debug)]
pub enum Error {
    /// The offset to the point data was too small.
    #[error("offset to the point data is too small: {0}")]
    OffsetToPointDataTooSmall(u32),

    /// The offset to the start of the evlrs is too small.
    #[error("offset to the start of the evlrs is too small: {0}")]
    OffsetToEvlrsTooSmall(u64),
}

#[inline]
pub(crate) fn read_point_from<R: std::io::Read>(
    mut source: &mut R,
    header: &Header,
) -> Result<Point> {
    let point = raw::Point::read_from(&mut source, header.point_format())
        .map(|raw_point| Point::new(raw_point, header.transforms()));
    point
}

/// Trait to specify behaviour a a PointReader
pub(crate) trait PointReader: Debug + Send {
    fn read_next(&mut self) -> Option<Result<Point>>;
    fn seek(&mut self, position: u64) -> Result<()>;
    fn header(&self) -> &Header;
}

/// An iterator over of the points in a `Reader`.
///
/// This struct is generally created by calling `points()` on `Reader`.
#[derive(Debug)]
pub struct PointIterator<'a> {
    point_reader: &'a mut dyn PointReader,
}

impl<'a> Iterator for PointIterator<'a> {
    type Item = Result<Point>;

    fn next(&mut self) -> Option<Self::Item> {
        self.point_reader.read_next()
    }
}

#[derive(Debug)]
struct UncompressedPointReader<R: std::io::Read + Seek> {
    source: R,
    header: Header,
    offset_to_point_data: u64,
    /// index of the last point read
    last_point_idx: u64,
}

impl<R: std::io::Read + Seek + Debug + Send> PointReader for UncompressedPointReader<R> {
    fn read_next(&mut self) -> Option<Result<Point>> {
        if self.last_point_idx < self.header.number_of_points() {
            self.last_point_idx += 1;
            Some(read_point_from(&mut self.source, &self.header))
        } else {
            None
        }
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        self.last_point_idx = position;
        self.source.seek(SeekFrom::Start(
            self.offset_to_point_data + position * u64::from(self.header.point_format().len()),
        ))?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}

/// A trait for objects which read LAS data.
pub trait Read {
    /// Returns a reference to this reader's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Read, Reader};
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let header = reader.header();
    /// ```
    fn header(&self) -> &Header;

    /// Reads a point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::{Read, Reader};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let point = reader.read().unwrap().unwrap();
    /// ```
    fn read(&mut self) -> Option<Result<Point>>;

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
    /// use las::{Read, Reader};
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
    /// # use las::{Read, Reader};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = reader.points().collect::<Result<Vec<_>, _>>().unwrap();
    /// ```
    fn points(&mut self) -> PointIterator;
}

/// Reads LAS data.
#[derive(Debug)]
pub struct Reader<'a> {
    point_reader: Box<dyn PointReader + 'a>,
}

impl<'a> Reader<'a> {
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
    pub fn new<R: std::io::Read + Seek + Send + Debug + 'a>(mut read: R) -> Result<Reader<'a>> {
        use std::io::Read;

        let raw_header = raw::Header::read_from(&mut read)?;
        let mut position = u64::from(raw_header.header_size);
        let number_of_variable_length_records = raw_header.number_of_variable_length_records;
        let offset_to_point_data = u64::from(raw_header.offset_to_point_data);
        let offset_to_end_of_points = raw_header.offset_to_end_of_points();
        let evlr = raw_header.evlr;

        let mut builder = Builder::new(raw_header)?;

        if !cfg!(feature = "laz") && builder.point_format.is_compressed {
            return Err(crate::Error::Laszip);
        }

        for _ in 0..number_of_variable_length_records {
            let vlr = raw::Vlr::read_from(&mut read, false).map(Vlr::new)?;
            position += vlr.len(false) as u64;
            builder.vlrs.push(vlr);
        }
        match position.cmp(&offset_to_point_data) {
            Ordering::Less => {
                read.by_ref()
                    .take(offset_to_point_data - position)
                    .read_to_end(&mut builder.vlr_padding)?;
            }
            Ordering::Equal => {} // pass
            Ordering::Greater => {
                return Err(Error::OffsetToPointDataTooSmall(offset_to_point_data as u32).into())
            }
        }

        read.seek(SeekFrom::Start(offset_to_end_of_points))?;
        if let Some(evlr) = evlr {
            match evlr.start_of_first_evlr.cmp(&offset_to_end_of_points) {
                Ordering::Less => {
                    return Err(Error::OffsetToEvlrsTooSmall(evlr.start_of_first_evlr).into())
                }
                Ordering::Equal => {} // pass
                Ordering::Greater => {
                    let n = evlr.start_of_first_evlr - offset_to_end_of_points;
                    read.by_ref()
                        .take(n)
                        .read_to_end(&mut builder.point_padding)?;
                }
            }
            builder
                .evlrs
                .push(raw::Vlr::read_from(&mut read, true).map(Vlr::new)?);
        }

        read.seek(SeekFrom::Start(offset_to_point_data))?;

        let header = builder.into_header()?;

        #[cfg(feature = "laz")]
        {
            if header.point_format().is_compressed {
                Ok(Reader {
                    point_reader: Box::new(CompressedPointReader::new(read, header)?),
                })
            } else {
                Ok(Reader {
                    point_reader: Box::new(UncompressedPointReader {
                        source: read,
                        header,
                        offset_to_point_data,
                        last_point_idx: 0,
                    }),
                })
            }
        }
        #[cfg(not(feature = "laz"))]
        {
            Ok(Reader {
                point_reader: Box::new(UncompressedPointReader {
                    source: read,
                    header,
                    offset_to_point_data,
                    last_point_idx: 0,
                }),
            })
        }
    }
}

impl<'a> Read for Reader<'a> {
    /// Returns a reference to this reader's header.
    fn header(&self) -> &Header {
        self.point_reader.header()
    }

    /// Reads a point.
    fn read(&mut self) -> Option<Result<Point>> {
        self.point_reader.read_next()
    }

    /// Seeks to the given point number, zero-indexed.
    fn seek(&mut self, position: u64) -> Result<()> {
        self.point_reader.seek(position)
    }

    /// Returns an iterator over this reader's points.
    fn points(&mut self) -> PointIterator {
        PointIterator {
            point_reader: &mut *self.point_reader,
        }
    }
}

impl<'a> Reader<'a> {
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
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader<'a>> {
        File::open(path)
            .map_err(crate::Error::from)
            .and_then(|file| Reader::new(BufReader::new(file)))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Write, Writer};

    use super::*;

    #[test]
    fn seek() {
        let mut writer = Writer::default();
        writer.write(Default::default()).unwrap();
        let point = Point {
            x: 1.,
            y: 2.,
            z: 3.,
            ..Default::default()
        };
        writer.write(point.clone()).unwrap();
        let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
        reader.seek(1).unwrap();
        assert_eq!(point, reader.read().unwrap().unwrap());
        assert!(reader.read().is_none());
    }
}
