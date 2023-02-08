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

use async_trait::async_trait;
use futures::io::{AsyncReadExt, AsyncSeek, AsyncSeekExt};
use std::io::SeekFrom;

#[cfg(feature = "laz")]
use crate::compression::CompressedPointReader;

use crate::{raw, Builder, Error, Header, Point, Result, Vlr};
use std::{cmp::Ordering, fmt::Debug};

#[inline]
pub(crate) async fn read_point_from<R: futures::io::AsyncRead + Unpin>(
    mut source: &mut R,
    header: &Header,
) -> Result<Point> {
    let point = raw::Point::read_from_async(&mut source, header.point_format())
        .await
        .map(|raw_point| Point::new(raw_point, header.transforms()));
    point
}

/// Trait to specify behaviour a a PointReader
#[async_trait]
pub(crate) trait PointReader: Debug + Send {
    async fn read_next(&mut self) -> Option<Result<Point>>;
    async fn seek(&mut self, position: u64) -> Result<()>;
    // XXX?
    fn header(&self) -> &Header;
}

/// An iterator over of the points in a `Reader`.
///
/// This struct is generally created by calling `points()` on `Reader`.
#[derive(Debug)]
pub struct PointIterator<'a> {
    point_reader: &'a mut dyn PointReader,
}

impl<'a> PointIterator<'a> {
    /// Iterator like next() method
    pub async fn next(&mut self) -> Option<Result<Point>> {
        self.point_reader.read_next().await
    }
}

/*
impl<'a> Iterator for PointIterator<'a> {
    type Item = Result<Point>;

    fn next(&mut self) -> Option<Self::Item> {
        self.point_reader.read_next()
    }
}
*/

#[derive(Debug)]
struct UncompressedPointReader<R: futures::io::AsyncRead + AsyncSeek + Unpin> {
    source: R,
    header: Header,
    offset_to_point_data: u64,
    /// index of the last point read
    last_point_idx: u64,
}

#[async_trait]
impl<R: futures::io::AsyncRead + AsyncSeek + Unpin + Debug + Send> PointReader
    for UncompressedPointReader<R>
{
    async fn read_next(&mut self) -> Option<Result<Point>> {
        if self.last_point_idx < self.header.number_of_points() {
            self.last_point_idx += 1;
            Some(read_point_from(&mut self.source, &self.header).await)
        } else {
            None
        }
    }

    async fn seek(&mut self, position: u64) -> Result<()> {
        use futures::io::AsyncSeekExt;

        self.last_point_idx = position;
        self.source
            .seek(SeekFrom::Start(
                self.offset_to_point_data + position * u64::from(self.header.point_format().len()),
            ))
            .await?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}

/// A trait for objects which read LAS data.
#[async_trait]
pub trait AsyncRead {
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
    async fn read(&mut self) -> Option<Result<Point>>;

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
    async fn seek(&mut self, position: u64) -> Result<()>;

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
pub struct AsyncReader<'a> {
    point_reader: Box<dyn PointReader + 'a>,
}

impl<'a> AsyncReader<'a> {
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
    pub async fn new<R: futures::io::AsyncRead + AsyncSeek + Unpin + Debug + Send + 'a>(
        mut read: R,
    ) -> Result<AsyncReader<'a>> {
        use std::io::Cursor;

        // Read fixed part
        let mut buf = [0; 227];
        read.read_exact(&mut buf).await?;

        let mut raw_header = raw::Header::read_from(Cursor::new(buf))?;
        let tail_length = raw_header.remaining_bytes_to_read();
        let tail = Vec::with_capacity(tail_length);
        raw_header.finish_parsing(Cursor::new(tail))?;

        let mut position = u64::from(raw_header.header_size);
        let number_of_variable_length_records = raw_header.number_of_variable_length_records;
        let offset_to_point_data = u64::from(raw_header.offset_to_point_data);
        let offset_to_end_of_points = raw_header.offset_to_end_of_points();
        let evlr = raw_header.evlr;

        let mut builder = Builder::new(raw_header)?;

        /*
        XXX
        if !cfg!(feature = "laz") && builder.point_format.is_compressed {
            return Err(crate::Error::Laszip);
        }
        */

        for _ in 0..number_of_variable_length_records {
            let vlr = raw::Vlr::read_from_async(&mut read, false)
                .await
                .map(Vlr::new)?;
            position += vlr.len(false) as u64;
            builder.vlrs.push(vlr);
        }
        match position.cmp(&offset_to_point_data) {
            Ordering::Less => {
                let mut take = read.take(offset_to_point_data - position);
                take.read_to_end(&mut builder.vlr_padding).await?;
                read = take.into_inner();
            }
            Ordering::Equal => {} // pass
            Ordering::Greater => {
                return Err(crate::reader::Error::OffsetToPointDataTooSmall(
                    offset_to_point_data as u32,
                )
                .into())
            }
        }

        read.seek(SeekFrom::Start(offset_to_end_of_points)).await?;
        if let Some(evlr) = evlr {
            match evlr.start_of_first_evlr.cmp(&offset_to_end_of_points) {
                Ordering::Less => {
                    return Err(crate::reader::Error::OffsetToEvlrsTooSmall(
                        evlr.start_of_first_evlr,
                    )
                    .into())
                }
                Ordering::Equal => {} // pass
                Ordering::Greater => {
                    let n = evlr.start_of_first_evlr - offset_to_end_of_points;
                    let mut take = read.take(n);
                    take.read_to_end(&mut builder.point_padding).await?;
                    read = take.into_inner();
                }
            }
            builder.evlrs.push(
                raw::Vlr::read_from_async(&mut read, true)
                    .await
                    .map(Vlr::new)?,
            );
        }

        read.seek(SeekFrom::Start(offset_to_point_data)).await?;

        let header = builder.into_header()?;

        //        #[cfg(feature = "laz")]
        //        {
        //            if header.point_format().is_compressed {
        //                Ok(Reader {
        //                    point_reader: Box::new(CompressedPointReader::new(read, header)?),
        //                })
        //            } else {
        //                Ok(Reader {
        //                    point_reader: Box::new(UncompressedPointReader {
        //                        source: read,
        //                        header,
        //                        offset_to_point_data,
        //                        last_point_idx: 0,
        //                    }),
        //                })
        //            }
        //        }
        #[cfg(not(feature = "laz"))]
        {
            Ok(AsyncReader {
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

#[async_trait]
impl<'a> AsyncRead for AsyncReader<'a> {
    /// Returns a reference to this reader's header.
    fn header(&self) -> &Header {
        self.point_reader.header()
    }

    /// Reads a point.
    async fn read(&mut self) -> Option<Result<Point>> {
        self.point_reader.read_next().await
    }

    /// Seeks to the given point number, zero-indexed.
    async fn seek(&mut self, position: u64) -> Result<()> {
        self.point_reader.seek(position).await
    }

    /// Returns an iterator over this reader's points.
    fn points(&mut self) -> PointIterator {
        PointIterator {
            point_reader: &mut *self.point_reader,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Write, Writer};

    use super::*;

    /*
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
        let mut reader = AsyncReader::new(writer.into_inner().unwrap()).unwrap();
        reader.seek(1).unwrap();
        assert_eq!(point, reader.read().unwrap().unwrap());
        assert!(reader.read().is_none());
    }
    */
}
