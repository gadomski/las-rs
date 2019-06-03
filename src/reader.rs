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

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Cursor};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};

use {Builder, Header, Point, raw, Result, Vlr};

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
    number_of_points: u64,
    number_read: u64,
    offset_to_point_data: u64,
    read: R,

    #[cfg(feature = "lazperf-compression")]
    vlr_decompressor: lazperf::VlrDecompressor,
    #[cfg(feature = "lazperf-compression")]
    raw_compressed_points: Vec<u8>,
    #[cfg(feature = "lazperf-compression")]
    tmp_point: Vec<u8>,

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
        let mut position = u64::from(raw_header.header_size);
        let number_of_variable_length_records = raw_header.number_of_variable_length_records;
        let offset_to_point_data = u64::from(raw_header.offset_to_point_data);
        let offset_to_end_of_points = raw_header.offset_to_end_of_points();
        let evlr = raw_header.evlr;

        let mut builder = Builder::new(raw_header)?;

        if !cfg!(feature = "lazperf-compression") && builder.point_format.is_compressed {
            return Err(::Error::Laszip);
        }

        for _ in 0..number_of_variable_length_records {
            let vlr = raw::Vlr::read_from(&mut read, false).and_then(Vlr::new)?;
            position += vlr.len(false) as u64;
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
            builder.evlrs.push(
                raw::Vlr::read_from(&mut read, true).and_then(
                    Vlr::new,
                )?,
            );
        }

        read.seek(SeekFrom::Start(offset_to_point_data))?;
        let header = builder.into_header()?;


        #[cfg(feature = "lazperf-compression")] {
            // skip offset to chunk table
            let offset_to_chunktable = read.read_u64::<LittleEndian>()?;
            let size_of_compressed_points = offset_to_chunktable - (offset_to_point_data + std::mem::size_of::<u64>() as u64);
            let mut raw_compressed_points = vec![0u8; size_of_compressed_points as usize];
            read.read_exact(&mut raw_compressed_points)?;
            let point_size = header.point_format().len() as usize;
            let tmp_point = vec![0u8; point_size];

            let mut laszip_vlr_data = None;
            for vlr in header.vlrs() {
                if &vlr.user_id == "laszip encoded" &&  vlr.record_id == 22204 {
                    laszip_vlr_data = Some(vlr.data.clone());
                }
            }
            let laszip_vlr_data = laszip_vlr_data.unwrap();
            let vlr_decompressor = lazperf::VlrDecompressor::new(&raw_compressed_points, point_size, &laszip_vlr_data);

            Ok(Reader {
                number_of_points: header.number_of_points(),
                header,
                offset_to_point_data,
                read,
                number_read: 0,
                vlr_decompressor,
                raw_compressed_points,
                tmp_point,
            })
        }
        #[cfg(not(feature = "lazperf-compression"))] {
            Ok(Reader {
                number_of_points: header.number_of_points(),
                header,
                offset_to_point_data,
                read,
                number_read: 0,
            })
        }
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
            #[cfg(feature = "lazperf-compression")]
            {
                self.vlr_decompressor.decompress_one_to(self.tmp_point.as_mut_slice());
                let mut read = Cursor::new(&self.tmp_point);
                let point = raw::Point::read_from(&mut read, self.header.point_format())
                    .map(|raw_point| {
                        Some(Point::new(raw_point, self.header.transforms()))
                    });
                self.number_read += 1;
                point
            }
            #[cfg(not(feature = "lazperf-compression"))]
            {
                let point = raw::Point::read_from(&mut self.read, self.header.point_format())
                    .map(|raw_point| {
                        Some(Point::new(raw_point, self.header.transforms()))
                    });
                self.number_read += 1;
                point
            }
        }
    }

    /// Seeks to the given point number, zero-indexed.
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
        self.read.seek(SeekFrom::Start(
            self.offset_to_point_data +
                position *
                    u64::from(self.header.point_format().len()),
        ))?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use Writer;

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
    }
}
