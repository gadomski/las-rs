//! Write points to a las file.

use std::io::Write;

use Result;
use reader::Reader;

pub struct Writer;

impl Writer {
    /// Creates a new writer for a `Write` stream.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cursor = std::io::Cursor(Vec::new());
    /// let writer = las::Writer::new(cursor);
    /// ```
    pub fn new<W: Write>(writer: W) -> Result<Writer> {
        Ok(Writer)
    }

    /// Writes all the points in a reader to this writer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut reader = las::Reader::open("data/1.2_0.las");
    /// let mut cursor = std::io::Cursor(Vec::new());
    /// let mut writer = las::Writer::new(cursor);
    /// let point_count = writer.write_from_reader(&mut reader);
    /// expect_eq!(1, point_count);
    /// ```
    pub fn write_from_reader(&mut self, reader: Reader) -> Result<u32> {
        Ok(0)
    }
}
