use {Error, Header, Point, Result};
use header::{GpsTimeType, WriteRawHeader};
use point::WriteRawPoint;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use vlr::WriteRawVlr;

/// Writes LAS data.
///
/// The LAS header needs to be re-written when the writer closes. For convenience, this is done via
/// the `Drop` implementation of the writer. One consequence is that if the header re-write fails
/// during the drop, a panic will result. If you want to check for errors instead of panicing, use
/// `close` explicitly.
///
/// ```
/// use std::io::Cursor;
/// # use las::Writer;
/// {
///     let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
///     writer.close().unwrap();
/// } // <- `close` is not called
/// ```
#[derive(Debug)]
pub struct Writer<W: Seek + Write> {
    closed: bool,
    header: Header,
    write: W,
}

impl<W: Seek + Write> Writer<W> {
    /// Creates a new writer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let writer = Writer::new(Cursor::new(Vec::new()), Default::default());
    /// ```
    pub fn new(mut write: W, mut header: Header) -> Result<Writer<W>> {
        if header.version == (1, 0) && header.file_source_id != 0 {
            return Err(Error::VersionDoesNotSupport(header.version, "file source id".to_string()));
        }
        if header.version == (1, 0) || header.version == (1, 1) {
            if header.point_format.has_color() {
                return Err(Error::VersionDoesNotSupport(header.version, "color".to_string()));
            }
            match header.gps_time_type {
                GpsTimeType::Standard => {
                    return Err(Error::VersionDoesNotSupport(header.version,
                                                            "GPS standard time".to_string()))
                }
                _ => {}
            }
        }
        header.bounds = Default::default();
        header.number_of_points = 0;
        header.number_of_points_by_return = [0; 5];
        if header.version == (1, 0) {
            header.vlr_padding = vec![0xDD, 0xCC];
        }
        header.to_raw_header().and_then(|raw_header| write.write_raw_header(&raw_header))?;
        for vlr in header.vlrs.iter() {
            vlr.to_raw_vlr().and_then(|raw_vlr| write.write_raw_vlr(&raw_vlr))?;
        }
        if !header.vlr_padding.is_empty() {
            write.write_all(&header.vlr_padding)?;
        }
        Ok(Writer {
            closed: false,
            header: header,
            write: write,
        })
    }

    /// Writes a point.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    ///
    /// let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
    /// writer.write(&Default::default()).unwrap();
    /// ```
    pub fn write(&mut self, point: &Point) -> Result<()> {
        self.write
            .write_raw_point(&point.to_raw_point(&self.header.transforms)?,
                             &self.header.point_format)?;
        self.header.number_of_points += 1;
        if point.return_number > 0 {
            self.header.number_of_points_by_return[point.return_number as usize - 1] += 1;
        }
        self.header.bounds.grow(point);
        Ok(())
    }

    /// Close this writer.
    ///
    /// A second call to close is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
    /// writer.close().unwrap();
    /// writer.close().unwrap(); // <- no-op
    ///
    pub fn close(&mut self) -> Result<()> {
        if !self.closed {
            self.write.seek(SeekFrom::Start(0))?;
            self.header
                .to_raw_header()
                .and_then(|raw_header| self.write.write_raw_header(&raw_header))?;
            self.closed = true;
        }
        Ok(())
    }
}

impl Writer<BufWriter<File>> {
    /// Creates a new writer for a path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Writer;
    /// let writer = Writer::from_path("/dev/null", Default::default());
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P, header: Header) -> Result<Writer<BufWriter<File>>> {
        File::create(path)
            .map_err(Error::from)
            .and_then(|file| Writer::new(BufWriter::new(file), header))
    }
}

impl<W: Seek + Write> Drop for Writer<W> {
    fn drop(&mut self) {
        if !self.closed {
            self.close().expect("Error when dropping the writer");
        }
    }
}

#[cfg(test)]
mod tests {

    use Header;

    use byteorder::{LittleEndian, ReadBytesExt};

    use std::io::Cursor;
    use super::*;

    #[test]
    fn las_1_0_point_data_start_signature() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let header = Header {
                version: (1, 0),
                vlrs: vec![Default::default()],
                ..Default::default()
            };
            let mut writer = Writer::new(&mut cursor, header).unwrap();
            writer.write(&Default::default()).unwrap();
        }
        cursor.set_position(281);
        assert_eq!(0xCCDD, cursor.read_u16::<LittleEndian>().unwrap());
    }
}
