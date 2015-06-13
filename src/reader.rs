//! Read points from las files.

use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

use num::FromPrimitive;

use Result;
use header::Header;
use point::Classification;
use point::Point;
use point::ScanDirection;

pub struct Reader<R: Read + Seek> {
    header: Header,
    reader: R,
}

impl<R: Read + Seek> Reader<R> {
    /// Creates a reader for a `Read` object.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let stream = std::fs::File::open("data/1.2_0.las").unwrap();
    /// let reader = Reader::new(stream);
    /// ```
    pub fn new(mut reader: R) -> Result<Reader<R>> {
        Ok(Reader {
            header: try!(Header::new(&mut reader)),
            reader: reader,
        })
    }

    /// Returns the `Header`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let reader = Reader::open("data/1.2_0.las").unwrap();
    /// let header = reader.header();
    /// assert_eq!(*b"LASF", header.file_signature);
    /// ```
    pub fn header(&self) -> &Header { &self.header }

    /// Returns a vector of all the points in the lasfile.
    ///
    /// Only use this method if you really do want to load all the points into memory at once.
    /// Otherwise, use the provided iterator methods to scan through the points in a more efficient
    /// manner.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let mut reader = Reader::open("data/1.2_0.las").unwrap();
    /// let points = reader.points().unwrap();
    /// assert_eq!(1, points.len());
    /// ```
    pub fn points(&mut self) -> Result<Vec<Point>> {
        Ok(try!(self.points_iter()).collect())
    }

    /// Creates an interator over this reader's points.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// # use las::point::Point;
    /// let mut reader = Reader::open("data/1.2_0.las").unwrap();
    /// let points: Vec<Point> = reader.points_iter().unwrap().collect();
    /// assert_eq!(1, points.len());
    /// ```
    pub fn points_iter(&mut self) -> Result<PointsIterator<R>> {
        try!(self.reader.seek(SeekFrom::Start(self.header.offset_to_point_data as u64)));
        Ok(PointsIterator {
            reader: self,
        })
    }

    /// Reads and returns the next point from the reader.
    fn next_point(&mut self) -> Result<Point> {
        let mut point: Point = Default::default();
        point.x = try!(self.reader.read_u32::<LittleEndian>()) as f64 * self.header.scale.x +
            self.header.offset.x;
        point.y = try!(self.reader.read_u32::<LittleEndian>()) as f64 * self.header.scale.y +
            self.header.offset.y;
        point.z = try!(self.reader.read_u32::<LittleEndian>()) as f64 * self.header.scale.z +
            self.header.offset.z;
        point.intensity = try!(self.reader.read_u16::<LittleEndian>());
        let byte = try!(self.reader.read_u8());
        point.return_number = byte & 0b00000111;
        point.number_of_returns = byte >> 3 & 0b00000111;
        point.scan_direction = match ScanDirection::from_u8(byte >> 6 & 0b00000001) {
            Some(scan_direction) => scan_direction,
            None => unreachable!(),
        };
        point.edge_of_flight_line = (byte >> 7 & 0b00000001) == 1;
        point.classification = match Classification::from_u8(try!(self.reader.read_u8())) {
            Some(classification) => classification,
            None => Default::default(),
        };
        point.scan_angle_rank = try!(self.reader.read_i8());
        point.user_data = try!(self.reader.read_u8());
        point.point_source_id = try!(self.reader.read_u16::<LittleEndian>());

        if self.header.point_data_format_id == 1 {
            point.gps_time = Some(try!(self.reader.read_f64::<LittleEndian>()));
        }

        Ok(point)
    }
}

impl Reader<File> {
    /// Opens a reader for a given file path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let reader = Reader::open("data/1.2_0.las");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Reader<File>> {
        // TODO wrap in BufRead
        let reader = try!(File::open(path));
        Ok(try!(Reader::new(reader)))
    }
}

/// Iterator over the points of a reader.
///
/// The iterator starts at the first point and reads through to the end.
pub struct PointsIterator<'a, R: 'a + Read + Seek> {
    reader: &'a mut Reader<R>,
}

impl<'a, R: Read + Seek> Iterator for PointsIterator<'a, R> {
    type Item = Point;

    fn next(&mut self) -> Option<Point> {
        self.reader.next_point().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use point::Classification;
    use point::Point;
    use point::ScanDirection;

    const POINT0: Point = Point {
        x: 470692.44,
        y: 4602888.90,
        z: 16.0,
        intensity: 0,
        return_number: 2,
        number_of_returns: 0,
        scan_direction: ScanDirection::Backward,
        edge_of_flight_line: false,
        classification: Classification::Ground,
        scan_angle_rank: -13,
        user_data: 0,
        point_source_id: 0,
        gps_time: None,
    };

    const POINT1: Point = Point {
        x: 470692.44,
        y: 4602888.90,
        z: 16.0,
        intensity: 0,
        return_number: 2,
        number_of_returns: 0,
        scan_direction: ScanDirection::Backward,
        edge_of_flight_line: false,
        classification: Classification::Ground,
        scan_angle_rank: -13,
        user_data: 0,
        point_source_id: 0,
        gps_time: Some(1205902800.0),
    };

    fn check_file(filename: &str, reference_point: &Point) {
        let mut reader = Reader::open(filename).unwrap();
        let point = &reader.points().unwrap()[0];
        assert_eq!(reference_point, point);
    }

    #[test]
    fn header() {
        let reader = Reader::open("data/1.2_0.las").unwrap();
        let header = reader.header();
        assert_eq!(*b"LASF", header.file_signature);
        assert_eq!(0, header.file_source_id);
        assert_eq!(0, header.global_encoding);
        assert_eq!("b8f18883-1baa-0841-bca3-6bc68e7b062e", header.project_id.as_string());
        assert_eq!(1, header.version_major);
        assert_eq!(2, header.version_minor);
        assert_eq!("libLAS", header.system_identifier);
        assert_eq!("libLAS 1.2", header.generating_software);
        assert_eq!(78, header.file_creation_day_of_year);
        assert_eq!(2008, header.file_creation_year);
        assert_eq!(227, header.header_size);
        assert_eq!(438, header.offset_to_point_data);
        assert_eq!(2, header.number_of_variable_length_records);
        assert_eq!(0, header.point_data_format_id);
        assert_eq!(20, header.point_data_record_length);
        assert_eq!(1, header.number_of_point_records);
        assert_eq!([0, 1, 0, 0, 0], header.number_of_points_by_return);
        assert_eq!(0.01, header.scale.x);
        assert_eq!(0.01, header.scale.y);
        assert_eq!(0.01, header.scale.z);
        assert_eq!(0.0, header.offset.x);
        assert_eq!(0.0, header.offset.y);
        assert_eq!(0.0, header.offset.z);
        assert_eq!(470692.447538, header.min.x);
        assert_eq!(4602888.904642, header.min.y);
        assert_eq!(16.0, header.min.z);
        assert_eq!(470692.447538, header.max.x);
        assert_eq!(4602888.904642, header.max.y);
        assert_eq!(16.0, header.max.z);
    }

    #[test]
    fn points() {
        let mut reader = Reader::open("data/1.2_0.las").unwrap();
        let points: Vec<Point> = reader.points_iter().unwrap().collect();
        assert_eq!(1, points.len());
    }

    #[test]
    fn formats_and_versions() {
        check_file("data/1.0_0.las", &POINT0);
        check_file("data/1.1_0.las", &POINT0);
        check_file("data/1.2_0.las", &POINT0);

        check_file("data/1.0_1.las", &POINT1);
        check_file("data/1.1_1.las", &POINT1);
        check_file("data/1.2_1.las", &POINT1);
    }
}
