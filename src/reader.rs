//! Read points from las files.

use std::io::Read;
use std::path::Path;

use Result;
use point::Point;

pub struct Reader;

impl Reader {
    /// Creates a reader for a `Read` object.
    ///
    /// # Examples
    ///
    /// ```
    /// let stream = std::fs::File::open("data/1.2_0.las");
    /// let reader = las::Reader::new(stream);
    /// ```
    pub fn new<R: Read>(reader: R) -> Result<Reader> {
        Ok(Reader)
    }

    /// Opens a reader for a given file path.
    ///
    /// # Examples
    ///
    /// ```
    /// let reader = las::Reader::open("data/1.2_0.las");
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Reader> {
        Ok(Reader)
    }

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
