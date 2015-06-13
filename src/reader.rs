//! Read points from las files.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use Result;
use header::Header;
use point::Point;

pub struct Reader {
    header: Header,
}

impl Reader {
    /// Creates a reader for a `Read` object.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream = std::fs::File::open("data/1.2_0.las");
    /// let reader = las::Reader::new(stream);
    /// ```
    pub fn new<R: Read>(mut reader: R) -> Result<Reader> {
        Ok(Reader {
            header: try!(Header::new(&mut reader)),
        })
    }

    /// Opens a reader for a given file path.
    ///
    /// # Examples
    ///
    /// ```
    /// let reader = las::Reader::open("data/1.2_0.las");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Reader> {
        // TODO wrap in BufRead
        let reader = try!(File::open(path));
        Ok(try!(Reader::new(reader)))
    }

    /// Returns the `Header`.
    ///
    /// # Examples
    ///
    /// ```
    /// let reader = las::Reader::open("data/1.2_0.las");
    /// let header = reader.header();
    /// assert_eq!("LASF", header.file_signature);
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
    /// let reader = las::Reader::open("data/1.2_0.las");
    /// let points = reader.points();
    /// expect_eq!(1, points.len());
    /// ```
    pub fn points(&mut self) -> Result<Vec<Point>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let mut reader = Reader::open("data/1.2_0.las").unwrap();
        let header = reader.header();
        assert_eq!(*b"LASF", header.file_signature);
        assert_eq!(0, header.file_source_id);
        assert_eq!(0, header.global_encoding);
        assert_eq!([0; 16], header.project_id);
        assert_eq!(1, header.version_major);
        assert_eq!(1, header.version_minor);
        assert_eq!("libLAS", header.system_identifier);
        assert_eq!("libLAS 1.2", header.generating_software);
        assert_eq!(0, header.file_creation_day_of_year);
        assert_eq!(0, header.file_creation_year);
        assert_eq!(277, header.header_size);
        assert_eq!(425, header.offset_to_point_data);
        assert_eq!(3, header.number_of_variable_length_records);
        assert_eq!(0, header.point_data_format_id);
        assert_eq!(0, header.point_data_record_length);
        assert_eq!(1, header.number_of_point_records);
        assert_eq!([1, 0, 0, 0, 0], header.number_of_points_by_return);
        assert_eq!(0.01, header.scale_factors.x);
        assert_eq!(0.01, header.scale_factors.y);
        assert_eq!(0.01, header.scale_factors.z);
        assert_eq!(0.0, header.offsets.x);
        assert_eq!(0.0, header.offsets.y);
        assert_eq!(0.0, header.offsets.z);
        assert_eq!(0.0, header.mins.x);
        assert_eq!(0.0, header.mins.y);
        assert_eq!(0.0, header.mins.z);
        assert_eq!(0.0, header.maxs.x);
        assert_eq!(0.0, header.maxs.y);
        assert_eq!(0.0, header.maxs.z);
    }
}
