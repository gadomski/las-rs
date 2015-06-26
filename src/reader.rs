//! Read points from las files.

use std::fs::File;
use std::io::BufReader;
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
use vlr::Vlr;

pub struct Reader<R: Read + Seek> {
    header: Header,
    reader: R,
    vlrs: Vec<Vlr>,
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
        let header = try!(Header::new(&mut reader));
        try!(reader.seek(SeekFrom::Start(header.header_size as u64)));
        let vlrs = try!(Vlr::read_n_from(&mut reader, header.number_of_variable_length_records as usize));
        try!(reader.seek(SeekFrom::Start(header.offset_to_point_data as u64)));
        Ok(Reader {
            header: header,
            reader: reader,
            vlrs: vlrs,
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

    /// Returns a `Vec` of the file's `Vlr`s.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let mut reader = Reader::open("data/1.2_0.las").unwrap();
    /// let vlrs = reader.vlrs();
    /// assert_eq!(2, vlrs.len());
    /// ```
    pub fn vlrs(&self) -> &Vec<Vlr> { &self.vlrs }

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
    pub fn points(self) -> Result<Vec<Point>> {
        Ok(self.into_iter().collect())
    }

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

        match self.header.point_data_format_id {
            1 | 3 => point.gps_time = Some(try!(self.reader.read_f64::<LittleEndian>())),
            _ => (),
        }

        match self.header.point_data_format_id {
            2 | 3 => {
                point.red = Some(try!(self.reader.read_u16::<LittleEndian>()));
                point.green = Some(try!(self.reader.read_u16::<LittleEndian>()));
                point.blue = Some(try!(self.reader.read_u16::<LittleEndian>()));
            },
            _ => (),
        }

        Ok(point)
    }
}

impl Reader<BufReader<File>> {
    /// Opens a reader for a given file path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::reader::Reader;
    /// let reader = Reader::open("data/1.2_0.las");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>> {
        let reader = try!(File::open(path));
        Ok(try!(Reader::new(BufReader::new(reader))))
    }
}

impl<R: Read + Seek> IntoIterator for Reader<R> {
    type Item = Point;
    type IntoIter = PointsIterator<R>;

    fn into_iter(self) -> Self::IntoIter {
        PointsIterator { reader: self } 
    }
}

/// Iterator over the points of a reader.
///
/// The iterator starts at the first point and reads through to the end.
pub struct PointsIterator<R: Read + Seek> {
    reader: Reader<R>,
}

impl<R: Read + Seek> Iterator for PointsIterator<R> {
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
        red: None,
        green: None,
        blue: None,
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
        red: None,
        green: None,
        blue: None,
    };

    const POINT2: Point = Point {
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
        red: Some(255),
        green: Some(12),
        blue: Some(234),
    };

    const POINT3: Point = Point {
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
        red: Some(255),
        green: Some(12),
        blue: Some(234),
    };

    fn check_file(filename: &str, reference_point: &Point) {
        let reader = Reader::open(filename).unwrap();
        let point = &reader.points().unwrap()[0];
        assert_eq!(reference_point, point);
    }

    #[test]
    fn points() {
        let reader = Reader::open("data/1.2_0.las").unwrap();
        let points: Vec<Point> = reader.into_iter().collect();
        assert_eq!(1, points.len());
    }

    #[test]
    fn vlrs() {
        let reader = Reader::open("data/1.2_0.las").unwrap();
        let vlrs = reader.vlrs();

        let vlr = &vlrs[0];
        assert_eq!(0, vlr.reserved);
        assert_eq!("LASF_Projection", vlr.user_id);
        assert_eq!(34735, vlr.record_id);
        assert_eq!(64, vlr.record_length_after_header);
        assert_eq!("", vlr.description);
    }

    #[test]
    fn las10_0() {
        check_file("data/1.0_0.las", &POINT0);
    }

    #[test]
    fn las10_1() {
        check_file("data/1.0_1.las", &POINT1);
    }

    #[test]
    fn las11_0() {
        check_file("data/1.1_0.las", &POINT0);
    }

    #[test]
    fn las11_1() {
        check_file("data/1.1_1.las", &POINT1);
    }

    #[test]
    fn las12_0() {
        check_file("data/1.2_0.las", &POINT0);
    }

    #[test]
    fn las12_1() {
        check_file("data/1.2_1.las", &POINT1);
    }

    #[test]
    fn las12_2() {
        check_file("data/1.2_2.las", &POINT2);
    }

    #[test]
    fn las12_3() {
        check_file("data/1.2_3.las", &POINT3);
    }
}
