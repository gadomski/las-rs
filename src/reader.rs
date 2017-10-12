//! Read las points.

use {Error, Header, Point, Result, Vlr};
use header::ReadRawHeader;
use point::ReadRawPoint;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use vlr::ReadRawVlr;

/// Reads LAS data.
#[derive(Debug)]
pub struct Reader<R: Read> {
    /// The `Header`, as read.
    pub header: Header,
    read: R,
}

impl<R: Read> Reader<R> {
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
        let raw_header = read.read_raw_header()?;
        if raw_header.is_compressed() {
            return Err(Error::Laszip);
        }
        let vlrs = (0..raw_header.number_of_variable_length_records)
            .map(|_| {
                read.read_raw_vlr().and_then(|raw_vlr| raw_vlr.into_vlr())
            })
            .collect::<Result<Vec<Vlr>>>()?;
        let position = vlrs.iter().fold(
            raw_header.header_size as u32,
            |acc, vlr| acc + vlr.len(),
        );
        let vlr_padding = if position < raw_header.offset_to_point_data {
            let mut bytes = vec![0; (raw_header.offset_to_point_data - position) as usize];
            read.read_exact(&mut bytes)?;
            bytes
        } else {
            Vec::new()
        };
        let header = raw_header.into_header(vlrs, vlr_padding)?;
        Ok(Reader {
            header: header,
            read: read,
        })
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
        self.read.read_raw_point(&self.header.point_format).map(
            |option| option.map(|raw_point| raw_point.into_point(&self.header.transforms)),
        )
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
