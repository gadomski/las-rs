//! Read a las file.
//!
//! We don't always need to get all the points into memory at once, so this interface enables
//! sequential access.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::iter::IntoIterator;
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};

use Result;
use header::Header;
use io::read_full;
use point::{Classification, NumberOfReturns, Point, ReturnNumber, ScanDirection};
use scale::scale;
use vlr::Vlr;

/// A reader of las points.
#[derive(Debug)]
pub struct Reader<R: Read> {
    header: Header,
    vlrs: Vec<Vlr>,
    reader: R,
    nread: u32,
}

impl Reader<BufReader<File>> {
    /// Opens a new reader for the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        let reader = BufReader::new(try!(File::open(path)));
        Reader::new(reader)
    }
}

impl<R: Read + Seek> Reader<R> {
    /// Creates a new reader for a given `Read` object, consuming the read.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use las::reader::Reader;
    /// let file = File::open("data/1.0_0.las").unwrap();
    /// let reader = Reader::new(file);
    /// ```
    pub fn new(mut reader: R) -> Result<Reader<R>> {
        let header = try!(Header::read_from(&mut reader));
        let mut vlrs = Vec::with_capacity(header.number_of_variable_length_records as usize);
        let _ = try!(reader.seek(SeekFrom::Start(header.header_size as u64)));
        for _ in 0..header.number_of_variable_length_records {
            vlrs.push(try!(Vlr::read_from(&mut reader)));
        }

        let _ = try!(reader.seek(SeekFrom::Start(header.offset_to_point_data as u64)));

        Ok(Reader {
            header: header,
            vlrs: vlrs,
            reader: reader,
            nread: 0,
        })
    }

    /// Returns the next point in this reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let point = reader.next_point().unwrap();
    /// assert!(point.is_some());
    /// let point = reader.next_point().unwrap();
    /// assert!(point.is_none());
    /// ```
    pub fn next_point(&mut self) -> Result<Option<Point>> {
        if self.eof() {
            return Ok(None);
        }
        let mut point = Point::new();
        let start = try!(self.reader.seek(SeekFrom::Current(0)));
        point.x = scale(try!(self.reader.read_i32::<LittleEndian>()),
                        self.header.x_scale_factor,
                        self.header.x_offset);
        point.y = scale(try!(self.reader.read_i32::<LittleEndian>()),
                        self.header.y_scale_factor,
                        self.header.y_offset);
        point.z = scale(try!(self.reader.read_i32::<LittleEndian>()),
                        self.header.z_scale_factor,
                        self.header.x_offset);
        point.intensity = try!(self.reader.read_u16::<LittleEndian>());
        let byte = try!(self.reader.read_u8());
        point.return_number = try!(ReturnNumber::from_u8(byte & 0b00000111));
        point.number_of_returns = try!(NumberOfReturns::from_u8((byte >> 3) & 0b00000111));
        point.scan_direction = ScanDirection::from((byte >> 6) & 0b00000001 == 1);
        point.edge_of_flight_line = byte >> 7 == 1;
        let byte = try!(self.reader.read_u8());
        point.classification = try!(Classification::from_u8(byte & 0b00011111));
        point.synthetic = (byte >> 5) & 0b00000001 == 1;
        point.key_point = (byte >> 6) & 0b00000001 == 1;
        point.withheld = byte >> 7 == 1;
        point.scan_angle_rank = try!(self.reader.read_i8());
        point.user_data = try!(self.reader.read_u8());
        point.point_source_id = try!(self.reader.read_u16::<LittleEndian>());
        if self.header.point_data_format.has_time() {
            point.gps_time = Some(try!(self.reader.read_f64::<LittleEndian>()));
        }
        if self.header.point_data_format.has_color() {
            point.red = Some(try!(self.reader.read_u16::<LittleEndian>()));
            point.green = Some(try!(self.reader.read_u16::<LittleEndian>()));
            point.blue = Some(try!(self.reader.read_u16::<LittleEndian>()));
        }
        let bytes_read = try!(self.reader.seek(SeekFrom::Current(0))) - start;

        if bytes_read < self.header.point_data_record_length as u64 {
            let mut buf =
                vec![0; (self.header.point_data_record_length as u64 - bytes_read) as usize];
            try!(read_full(&mut self.reader, &mut buf[..]));
            point.extra_bytes = Some(buf);
        }

        self.nread += 1;

        Ok(Some(point))
    }
}

impl<R: Read> Reader<R> {
    /// Returns this reader's las header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let header = reader.header();
    pub fn header(&self) -> Header {
        self.header
    }

    /// Returns a reference to this reader's vlrs.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let vlrs = reader.vlrs();
    pub fn vlrs(&self) -> &Vec<Vlr> {
        &self.vlrs
    }

    /// Returns the number of points in this lasfile, as specified by the header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// assert_eq!(1, reader.npoints());
    /// ```
    pub fn npoints(&self) -> u32 {
        self.header.number_of_point_records
    }

    /// Returns true if this reader is at the end of the file.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::reader::Reader;
    /// let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// assert!(!reader.eof());
    /// let _ = reader.next_point().unwrap();
    /// assert!(reader.eof());
    /// ```
    pub fn eof(&self) -> bool {
        self.nread == self.npoints()
    }
}

impl<R: Read + Seek> IntoIterator for Reader<R> {
    type Item = Point;
    type IntoIter = ReaderIterator<R>;
    fn into_iter(self) -> Self::IntoIter {
        ReaderIterator { reader: self }
    }
}

/// An iterator over the reader's points.
///
/// # Panics
///
/// This iterator will panic if something goes wrong while reading the points. If you need to catch
/// those errors, use `Reader::next_point`.
#[derive(Debug)]
pub struct ReaderIterator<R: Read + Seek> {
    reader: Reader<R>,
}

impl<R: Read + Seek> Iterator for ReaderIterator<R> {
    type Item = Point;
    fn next(&mut self) -> Option<Self::Item> {
        self.reader.next_point().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reader_as_iter() {
        let reader = Reader::from_path("data/1.0_0.las").unwrap();
        let points: Vec<_> = reader.into_iter().collect();
        assert_eq!(1, points.len());
    }
}
