use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{TimeZone, UTC};

use {Error, Result};
use global_encoding::GlobalEncoding;
use header::Header;
use point::{Classification, Color, Format, NumberOfReturns, Point, ReturnNumber, ScanDirection,
            utils};
use utils::{Bounds, Triple};
use version::Version;
use vlr::Vlr;

/// Takes bytes and turns them into points and associated metadata.
#[derive(Debug)]
pub struct Reader<R> {
    /// LAS header.
    pub header: Header,
    read: R,
}

impl Reader<BufReader<File>> {
    /// Creates a reader for a file at the given path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        File::open(path).map_err(Error::from).and_then(|file| Reader::new(BufReader::new(file)))
    }
}

impl<R: Read + Seek> Reader<R> {
    /// Creates a new reader from a `Read` object.
    ///
    /// While `Reader::from_path` wraps the underlying `File` in a `BufReader`, this method does no
    /// such work for you. If you're planning on doing lots of reads, you should probably wrap your
    /// `Read` in a `BufReader` for performance reasons.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Cursor, Read};
    /// use std::fs::File;
    /// # use las::Reader;
    /// let mut buf = Vec::new();
    /// File::open("data/1.0_0.las").unwrap().read_to_end(&mut buf).unwrap();
    /// let reader = Reader::new(Cursor::new(buf));
    /// ```
    pub fn new(mut read: R) -> Result<Reader<R>> {
        let mut file_signature = String::new();
        try!((&mut read).take(4).read_to_string(&mut file_signature));
        if file_signature != "LASF" {
            return Err(Error::InvalidFileSignature(file_signature));
        }
        let file_source_id = try!(read.read_u16::<LittleEndian>());
        let global_encoding = try!(read.read_u16::<LittleEndian>());
        let mut project_id = [0; 16];
        try!(read.read_exact(&mut project_id));
        let version = Version::new(try!(read.read_u8()), try!(read.read_u8()));

        if !version.has_file_source_id() && file_source_id != 0 {
            return Err(Error::ReservedIsNotZero);
        }
        let file_source_id = if version.has_file_source_id() {
            Some(file_source_id)
        } else if file_source_id == 0 {
            None
        } else {
            return Err(Error::ReservedIsNotZero);
        };
        let global_encoding = if version.has_global_encoding() {
            Some(GlobalEncoding::from(global_encoding))
        } else if global_encoding == 0 {
            None
        } else {
            return Err(Error::ReservedIsNotZero);
        };

        let mut system_id = [0; 32];
        try!(read.read_exact(&mut system_id));
        let mut generating_software = [0; 32];
        try!(read.read_exact(&mut generating_software));
        let day = try!(read.read_u16::<LittleEndian>());
        let year = try!(read.read_u16::<LittleEndian>());
        let file_creation_date = UTC.yo(year as i32, day as u32);
        let header_size = try!(read.read_u16::<LittleEndian>());
        let offset_to_data = try!(read.read_u32::<LittleEndian>());
        let num_vlrs = try!(read.read_u32::<LittleEndian>());
        let point_format = Format::from(try!(read.read_u8()));
        if !point_format.is_supported() {
            return Err(Error::UnsupportedPointFormat(point_format));
        }
        let point_data_record_length = try!(read.read_u16::<LittleEndian>());
        let extra_bytes: i32 = point_data_record_length as i32 -
                               point_format.record_length() as i32;
        if extra_bytes < 0 {
            return Err(Error::InvalidPointDataRecordLength(point_format, point_data_record_length));
        }
        let point_count = try!(read.read_u32::<LittleEndian>());
        let mut point_count_by_return = [0; 5];
        for entry in point_count_by_return.iter_mut() {
            *entry = try!(read.read_u32::<LittleEndian>());
        }
        let scale = Triple {
            x: try!(read.read_f64::<LittleEndian>()),
            y: try!(read.read_f64::<LittleEndian>()),
            z: try!(read.read_f64::<LittleEndian>()),
        };
        let offset = Triple {
            x: try!(read.read_f64::<LittleEndian>()),
            y: try!(read.read_f64::<LittleEndian>()),
            z: try!(read.read_f64::<LittleEndian>()),
        };
        let maxx = try!(read.read_f64::<LittleEndian>());
        let minx = try!(read.read_f64::<LittleEndian>());
        let maxy = try!(read.read_f64::<LittleEndian>());
        let miny = try!(read.read_f64::<LittleEndian>());
        let maxz = try!(read.read_f64::<LittleEndian>());
        let minz = try!(read.read_f64::<LittleEndian>());
        let bounds = Bounds::new(minx, miny, minz, maxx, maxy, maxz);

        let vlrs = try!((0..num_vlrs)
            .map(|_| {
                let mut vlr: Vlr = Default::default();
                try!(read.read_u16::<LittleEndian>()); // reserved
                try!(read.read_exact(&mut vlr.user_id));
                vlr.record_id = try!(read.read_u16::<LittleEndian>());
                vlr.record_length = try!(read.read_u16::<LittleEndian>());
                try!(read.read_exact(&mut vlr.description));
                try!((&mut read).take(vlr.record_length as u64).read_to_end(&mut vlr.data));
                Ok(vlr)
            })
            .collect::<Result<Vec<Vlr>>>());

        try!(read.seek(SeekFrom::Start(offset_to_data as u64)));

        Ok(Reader {
            header: Header {
                file_source_id: file_source_id,
                global_encoding: global_encoding,
                project_id: project_id,
                version: version,
                system_id: system_id,
                generating_software: generating_software,
                header_size: header_size,
                file_creation_date: file_creation_date,
                point_format: point_format,
                extra_bytes: extra_bytes as u16,
                point_count: point_count,
                point_count_by_return: point_count_by_return,
                scale: scale,
                offset: offset,
                bounds: bounds,
                padding: offset_to_data -
                         vlrs.iter().fold(header_size as u32, |acc, vlr| acc + vlr.len()),
                vlrs: vlrs,
            },
            read: read,
        })
    }

    /// Reads a point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// // This reader has one point.
    /// let point = reader.read().unwrap().unwrap();
    /// assert!(reader.read().unwrap().is_none());
    /// ```
    pub fn read(&mut self) -> Result<Option<Point>> {
        let x = match self.read.read_i32::<LittleEndian>() {
            Ok(n) => n as f64 * self.header.scale.x + self.header.offset.x,
            Err(err) => {
                return if err.kind() == io::ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(Error::from(err))
                }
            }
        };
        let y = try!(self.read.read_i32::<LittleEndian>()) as f64 * self.header.scale.y +
                self.header.offset.y;
        let z = try!(self.read.read_i32::<LittleEndian>()) as f64 * self.header.scale.z +
                self.header.offset.z;
        let intensity = try!(self.read.read_u16::<LittleEndian>());
        let byte = try!(self.read.read_u8());
        let return_number = ReturnNumber::from(byte);
        let number_of_returns = NumberOfReturns::from(byte);
        let scan_direction = ScanDirection::from(byte);
        let edge_of_flight_line = utils::edge_of_flight_line(byte);
        let classification = Classification::from(try!(self.read.read_u8()), self.header.version);
        let scan_angle_rank = try!(self.read.read_i8());
        let user_data = try!(self.read.read_u8());
        let point_source_id = try!(self.read.read_u16::<LittleEndian>());
        let gps_time = if self.header.point_format.has_gps_time() {
            Some(try!(self.read.read_f64::<LittleEndian>()))
        } else {
            None
        };
        let color = if self.header.point_format.has_color() {
            let red = try!(self.read.read_u16::<LittleEndian>());
            let green = try!(self.read.read_u16::<LittleEndian>());
            let blue = try!(self.read.read_u16::<LittleEndian>());
            Some(Color {
                red: red,
                green: green,
                blue: blue,
            })
        } else {
            None
        };
        let mut extra_bytes = Vec::new();
        if self.header.extra_bytes > 0 {
            try!((&mut self.read)
                .take(self.header.extra_bytes as u64)
                .read_to_end(&mut extra_bytes));
        }
        Ok(Some(Point {
            x: x,
            y: y,
            z: z,
            intensity: intensity,
            return_number: return_number,
            number_of_returns: number_of_returns,
            scan_direction: scan_direction,
            edge_of_flight_line: edge_of_flight_line,
            classification: classification,
            scan_angle_rank: scan_angle_rank,
            user_data: user_data,
            point_source_id: point_source_id,
            gps_time: gps_time,
            color: color,
            extra_bytes: extra_bytes,
        }))
    }

    /// Creates a vector with all the points in this lasfile.
    ///
    /// If any of the reads causes an error, returns that error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let points = reader.read_to_end().unwrap();
    /// ```
    pub fn read_to_end(&mut self) -> Result<Vec<Point>> {
        self.iter_mut().collect()
    }

    /// Returns an iterator over points.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let points = reader.iter_mut().collect::<Result<Vec<_>, _>>().unwrap();
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<R> {
        IterMut { reader: self }
    }
}

/// Mutable iterator over points.
#[derive(Debug)]
pub struct IterMut<'a, R: 'a> {
    reader: &'a mut Reader<R>,
}

impl<'a, R: Read + Seek> Iterator for IterMut<'a, R> {
    type Item = Result<Point>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read() {
            Ok(Some(point)) => Some(Ok(point)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::io::{Cursor, Read, Seek};

    use point::{Point, ScanDirection};
    use utils::{ToLasStr, Triple};
    use version::Version;

    fn check_point(point: &Point) {
        assert!((point.x - 470692.44).abs() < 1e-2);
        assert!((point.y - 4602888.90).abs() < 1e-2);
        assert!((point.z - 16.00).abs() < 1e-2);
        assert_eq!(2, point.return_number);
        assert_eq!(0, point.number_of_returns);
        assert_eq!(0, point.intensity);
        assert_eq!(ScanDirection::from(0), point.scan_direction);
        assert_eq!(-13, point.scan_angle_rank);
        assert_eq!(2, point.classification);
    }

    fn check_0<T: Read + Seek>(reader: &mut Reader<T>) {
        let point = reader.read().unwrap().unwrap();
        check_point(&point);
        assert!(point.gps_time.is_none());
        assert!(point.color.is_none());
        assert!(reader.read().unwrap().is_none());
    }

    fn check_1<T: Read + Seek>(reader: &mut Reader<T>) {
        let point = reader.read().unwrap().unwrap();
        check_point(&point);
        assert_eq!(1205902800., point.gps_time.unwrap());
        assert!(point.color.is_none());
        assert!(reader.read().unwrap().is_none());
    }

    fn check_2<T: Read + Seek>(reader: &mut Reader<T>) {
        let point = reader.read().unwrap().unwrap();
        check_point(&point);
        assert!(point.gps_time.is_none());
        let color = point.color.unwrap();
        assert_eq!(255, color.red);
        assert_eq!(12, color.green);
        assert_eq!(234, color.blue);
        assert!(reader.read().unwrap().is_none());
    }

    fn check_3<T: Read + Seek>(reader: &mut Reader<T>) {
        let point = reader.read().unwrap().unwrap();
        check_point(&point);
        assert_eq!(1205902800., point.gps_time.unwrap());
        let color = point.color.unwrap();
        assert_eq!(255, color.red);
        assert_eq!(12, color.green);
        assert_eq!(234, color.blue);
        assert!(reader.read().unwrap().is_none());
    }

    fn las_vec(path: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        File::open(path).unwrap().read_to_end(&mut buf).unwrap();
        buf
    }

    #[test]
    fn reader_notafile() {
        assert!(Reader::from_path("data/1.0_0.txt").is_err());
    }

    #[test]
    fn reader_bad_file_signature() {
        for i in 0..4 {
            let mut buf = las_vec("data/1.0_0.las");
            buf[i] += 1;
            assert!(Reader::new(Cursor::new(buf)).is_err());
        }
    }

    #[test]
    fn reader_1_0_bad_reserved() {
        for i in 4..8 {
            let mut buf = las_vec("data/1.0_0.las");
            buf[i] += 1;
            assert!(Reader::new(Cursor::new(buf)).is_err());
        }
    }

    #[test]
    fn reader_1_1_bad_reserved() {
        for i in 4..6 {
            let mut buf = las_vec("data/1.1_0.las");
            buf[i] += 1;
            assert!(Reader::new(Cursor::new(buf)).is_ok());
        }
        for i in 6..8 {
            let mut buf = las_vec("data/1.1_0.las");
            buf[i] += 1;
            assert!(Reader::new(Cursor::new(buf)).is_err());
        }
    }

    #[test]
    fn reader_from_cursor() {
        check_0(&mut Reader::new(Cursor::new(las_vec("data/1.0_0.las"))).unwrap());
    }

    #[test]
    fn reader_1_0_0() {
        check_0(&mut Reader::from_path("data/1.0_0.las").unwrap());
    }

    #[test]
    fn reader_1_0_1() {
        check_1(&mut Reader::from_path("data/1.0_1.las").unwrap());
    }

    #[test]
    fn reader_1_1_0() {
        check_0(&mut Reader::from_path("data/1.1_0.las").unwrap());
    }

    #[test]
    fn reader_1_1_1() {
        check_1(&mut Reader::from_path("data/1.1_1.las").unwrap());
    }

    #[test]
    fn reader_1_2_0() {
        check_0(&mut Reader::from_path("data/1.2_0.las").unwrap());
    }

    #[test]
    fn reader_1_2_1() {
        check_1(&mut Reader::from_path("data/1.2_1.las").unwrap());
    }

    #[test]
    fn reader_1_2_2() {
        check_2(&mut Reader::from_path("data/1.2_2.las").unwrap());
    }

    #[test]
    fn reader_1_2_3() {
        check_3(&mut Reader::from_path("data/1.2_3.las").unwrap());
    }

    #[test]
    fn reader_point_count() {
        assert_eq!(1,
                   Reader::from_path("data/1.0_0.las").unwrap().header.point_count);
    }

    #[test]
    fn reader_bounds() {
        let reader = Reader::from_path("data/1.0_0.las").unwrap();
        let bounds = reader.header.bounds;
        assert!((bounds.min.x - 470692.44).abs() < 1e-2);
        assert!((bounds.min.y - 4602888.90).abs() < 1e-2);
        assert!((bounds.min.z - 16.00).abs() < 1e-2);
        assert!((bounds.max.x - 470692.44).abs() < 1e-2);
        assert!((bounds.max.y - 4602888.90).abs() < 1e-2);
        assert!((bounds.max.z - 16.00).abs() < 1e-2);
    }

    #[test]
    fn reader_version() {
        assert_eq!(Version::new(1, 0),
                   Reader::from_path("data/1.0_0.las").unwrap().header.version);
        assert_eq!(Version::new(1, 1),
                   Reader::from_path("data/1.1_0.las").unwrap().header.version);
        assert_eq!(Version::new(1, 2),
                   Reader::from_path("data/1.2_0.las").unwrap().header.version);
    }

    #[test]
    fn reader_system_id() {
        assert_eq!("libLAS",
                   Reader::from_path("data/1.0_0.las")
                       .unwrap()
                       .header
                       .system_id
                       .to_las_str()
                       .unwrap());
    }

    #[test]
    fn reader_generating_software() {
        assert_eq!("libLAS 1.2",
                   Reader::from_path("data/1.0_0.las")
                       .unwrap()
                       .header
                       .generating_software
                       .to_las_str()
                       .unwrap());
    }

    #[test]
    fn reader_read_to_end() {
        let points = Reader::from_path("data/1.0_0.las").unwrap().read_to_end().unwrap();
        assert_eq!(1, points.len());
    }

    #[test]
    fn reader_offset() {
        let offset = Reader::from_path("data/1.0_0.las").unwrap().header.offset;
        assert_eq!(Triple::new(0., 0., 0.), offset);
    }

    #[test]
    fn reader_vlrs() {
        let vlrs = Reader::from_path("data/1.0_0.las").unwrap().header.vlrs;
        assert_eq!(2, vlrs.len());
        let vlr = &vlrs[0];
        assert_eq!("LASF_Projection", vlr.user_id.to_las_str().unwrap());
        assert_eq!(34735, vlr.record_id);
        assert_eq!(64, vlr.record_length);
        assert_eq!("", vlr.description.to_las_str().unwrap());
        let vlr = &vlrs[1];
        assert_eq!("LASF_Projection", vlr.user_id.to_las_str().unwrap());
        assert_eq!(34737, vlr.record_id);
        assert_eq!(39, vlr.record_length);
        assert_eq!("", vlr.description.to_las_str().unwrap());
    }

    #[test]
    fn reader_extra_bytes() {
        let mut reader = Reader::from_path("data/1.2_1_extra_bytes.las").unwrap();
        let points = reader.read_to_end().unwrap();
        assert_eq!(43, points.len());
    }
}
