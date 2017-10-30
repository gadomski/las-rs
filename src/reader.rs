//! Read las points.

use {Error, Header, Point, Result, Vlr};
use raw;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Reads LAS data.
#[derive(Debug)]
pub struct Reader<R> {
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
        if raw_header.is_compressed() {
            return Err(Error::Laszip);
        }
        let mut vlrs = Vec::new();
        let mut position = raw_header.header_size as usize;
        for _ in 0..raw_header.number_of_variable_length_records {
            let vlr = raw::Vlr::read_from(&mut read, false).and_then(Vlr::new)?;
            position += vlr.len();
            vlrs.push(vlr);
        }
        let vlr_padding = if position < raw_header.offset_to_point_data as usize {
            let mut bytes = vec![0; raw_header.offset_to_point_data as usize - position];
            read.read_exact(&mut bytes)?;
            bytes
        } else {
            Vec::new()
        };
        if let Some(evlr) = raw_header.evlr {
            read.seek(SeekFrom::Start(evlr.start_of_first_evlr))?;
            vlrs.push(raw::Vlr::read_from(&mut read, true).and_then(Vlr::new)?);
        }
        read.seek(SeekFrom::Start(
            u64::from(raw_header.offset_to_point_data),
        ))?;
        let header = Header::new(raw_header, vlrs, vlr_padding)?;
        Ok(Reader {
            number_of_points: header.number_of_points,
            header: header,
            read: read,
            number_read: 0,
        })
    }
}

impl<R: Read> Reader<R> {
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
            let point = raw::Point::read_from(&mut self.read, self.header.point_format)
                .map(|raw_point| {
                    Some(Point::new(raw_point, self.header.transforms))
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
        File::open(path).map_err(Error::from).and_then(|file| {
            Reader::new(BufReader::new(file))
        })
    }
}

/// An iterator over of the points in a `Reader`.
///
/// This struct is generally created by calling `points()` on `Reader`.
#[derive(Debug)]
pub struct Points<'a, R: 'a + Read> {
    reader: &'a mut Reader<R>,
}

impl<'a, R: Read> Iterator for Points<'a, R> {
    type Item = Result<Point>;
    fn next(&mut self) -> Option<Result<Point>> {
        match self.reader.read() {
            Ok(None) => None,
            Ok(Some(point)) => Some(Ok(point)),
            Err(err) => Some(Err(err)),
        }
    }
}
