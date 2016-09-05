use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use {Error, Result};
use header::{Header, ReadHeader};
use point::{Point, ReadPoint};
use vlr::{ReadVlr, Vlr};

/// Takes bytes and turns them into points and associated metadata.
#[derive(Debug)]
pub struct Reader<R> {
    /// LAS header.
    pub header: Header,
    /// Variable length records.
    pub vlrs: Vec<Vlr>,
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
        let header = try!(read.read_header());
        let vlrs = try!((0..header.num_vlrs).map(|_| read.read_vlr()).collect());
        try!(read.seek(SeekFrom::Start(header.offset_to_point_data as u64)));
        Ok(Reader {
            header: header,
            vlrs: vlrs,
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
        self.read.read_point(self.header.transforms,
                             self.header.point_format,
                             self.header.version,
                             self.header.extra_bytes)
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
    use utils::ToLasStr;
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
    fn reader_vlrs() {
        let vlrs = Reader::from_path("data/1.0_0.las").unwrap().vlrs;
        assert_eq!(2, vlrs.len());
        let vlr = &vlrs[0];
        assert_eq!("LASF_Projection", vlr.user_id.to_las_str().unwrap());
        assert_eq!(34735, vlr.record_id);
        assert_eq!("", vlr.description.to_las_str().unwrap());
        let vlr = &vlrs[1];
        assert_eq!("LASF_Projection", vlr.user_id.to_las_str().unwrap());
        assert_eq!(34737, vlr.record_id);
        assert_eq!("", vlr.description.to_las_str().unwrap());
    }

    #[test]
    fn reader_extra_bytes() {
        let mut reader = Reader::from_path("data/1.2_1_extra_bytes.las").unwrap();
        let points = reader.read_to_end().unwrap();
        assert_eq!(43, points.len());
    }
}
