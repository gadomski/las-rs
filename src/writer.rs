use {Header, Point, Result, Version};
use point::Format;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

quick_error! {
    /// Writer errors.
    #[derive(Debug)]
    pub enum Error {
        /// The writer is closed.
        Closed {
            description("the writer is closed")
        }
        /// Format is not supported by version.
        Format(version: Version, format: Format) {
            description("format is not supported by version")
            display("format {} is not supported by version {}", format, version)
        }
        /// The point format has color, but the point doesn't.
        MissingColor(format: Format, point: Point) {
            description("the point format has color, but the point doesn't")
            display("point format {} has color, but point {:?} doesn't", format, point)
        }
        /// The point format has extra bytes, but the point doesn't.
        MissingExtraBytes(format: Format, point: Point) {
            description("the point format has extra bytes, but the point doesn't")
            display("point format {} has extra bytes, but point {:?} doesn't", format, point)
        }
        /// The point format has gps time, but the point doesn't.
        MissingGpsTime(format: Format, point: Point) {
            description("the point format has gps time, but the point doesn't")
            display("point format {} has gps time, but point {:?} doesn't", format, point)
        }
        /// The point format has nir, but the point doesn't.
        MissingNir(format: Format, point: Point) {
            description("the point format has nir, but the point doesn't")
            display("point format {} has nir, but point {:?} doesn't", format, point)
        }
        /// The point format has waveform, but the point doesn't.
        MissingWaveform(format: Format, point: Point) {
            description("the point format has waveform, but the point doesn't")
            display("point format {} has waveform, but point {:?} doesn't", format, point)
        }
    }
}

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
    /// The header that is passed in will have various fields zero'd, e.g. bounds, number of
    /// points, etc.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Writer;
    /// let writer = Writer::new(Cursor::new(Vec::new()), Default::default());
    /// ```
    pub fn new(mut write: W, mut header: Header) -> Result<Writer<W>> {
        use feature::{Evlrs, FileSourceId, GpsStandardTime};

        if header.file_source_id != 0 {
            header.version.verify_support_for::<FileSourceId>()?;
        }
        if header.gps_time_type.is_standard() {
            header.version.verify_support_for::<GpsStandardTime>()?;
        }
        if !header.version.supports_point_format(header.point_format) {
            return Err(Error::Format(header.version, header.point_format).into());
        }
        // TODO check waveforms
        if header.evlrs().len() > 0 {
            header.version.verify_support_for::<Evlrs>()?;
        }

        header.bounds = Default::default();
        header.number_of_points = 0;
        header.number_of_points_by_return = HashMap::new();
        if header.version.requires_point_data_start_signature() {
            header.vlr_padding = ::raw::POINT_DATA_START_SIGNATURE.to_vec();
        }
        header.to_raw().and_then(
            |raw_header| raw_header.write_to(&mut write),
        )?;
        for vlr in header.vlrs().iter() {
            vlr.to_raw(false).and_then(
                |raw_vlr| raw_vlr.write_to(&mut write),
            )?;
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
    /// writer.write(Default::default()).unwrap();
    /// ```
    pub fn write(&mut self, point: Point) -> Result<()> {
        if self.closed {
            return Err(Error::Closed.into());
        }
        if self.header.point_format.has_gps_time && point.gps_time.is_none() {
            return Err(
                Error::MissingGpsTime(self.header.point_format, point).into(),
            );
        }
        if self.header.point_format.has_color && point.color.is_none() {
            return Err(Error::MissingColor(self.header.point_format, point).into());
        }
        if self.header.point_format.has_nir && point.nir.is_none() {
            return Err(Error::MissingNir(self.header.point_format, point).into());
        }
        if self.header.point_format.has_waveform && point.waveform.is_none() {
            return Err(
                Error::MissingWaveform(self.header.point_format, point).into(),
            );
        }
        if self.header.point_format.extra_bytes as usize != point.extra_bytes.len() {
            return Err(
                Error::MissingExtraBytes(self.header.point_format, point).into(),
            );
        }
        self.header.number_of_points += 1;
        {
            let entry = self.header
                .number_of_points_by_return
                .entry(point.return_number)
                .or_insert(0);
            *entry += 1;
        }
        self.header.bounds.grow(&point);
        point.into_raw(self.header.transforms).and_then(
            |raw_point| {
                raw_point.write_to(&mut self.write, self.header.point_format)
            },
        )?;
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
            // TODO evlr padding?
            for raw_evlr in self.header.evlrs().into_iter().map(
                |evlr| evlr.to_raw(true),
            )
            {
                raw_evlr?.write_to(&mut self.write)?;
            }
            self.write.seek(SeekFrom::Start(0))?;
            self.header.to_raw().and_then(|raw_header| {
                raw_header.write_to(&mut self.write)
            })?;
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
        File::create(path).map_err(::Error::from).and_then(|file| {
            Writer::new(BufWriter::new(file), header)
        })
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
    use super::*;
    use Header;
    use byteorder::{LittleEndian, ReadBytesExt};
    use point::Format;
    use std::io::Cursor;

    fn writer(format: Format, version: Version) -> Writer<Cursor<Vec<u8>>> {
        Writer::new(
            Cursor::new(Vec::new()),
            Header {
                point_format: format,
                version: version,
                ..Default::default()
            },
        ).unwrap()
    }

    #[test]
    fn las_1_0_point_data_start_signature() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let header = Header {
                version: (1, 0).into(),
                vlrs: vec![Default::default()],
                ..Default::default()
            };
            let mut writer = Writer::new(&mut cursor, header).unwrap();
            writer.write(Default::default()).unwrap();
        }
        cursor.set_position(281);
        assert_eq!(0xCCDD, cursor.read_u16::<LittleEndian>().unwrap());
    }

    #[test]
    fn already_closed() {
        let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
        writer.close().unwrap();
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_extra_bytes() {
        let format = Format {
            extra_bytes: 1,
            ..Default::default()
        };
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_gps_time() {
        let format = Format::new(1).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_color() {
        let format = Format::new(2).unwrap();
        let mut writer = writer(format, Version::new(1, 2));
        assert!(writer.write(Default::default()).is_err());
    }

    #[test]
    fn missing_nir() {
        let format = Format::new(8).unwrap();
        let mut writer = writer(format, Version::new(1, 4));
        let point = Point {
            gps_time: Some(0.),
            color: Some(Default::default()),
            ..Default::default()
        };
        assert!(writer.write(point).is_err());
    }

    #[test]
    fn missing_waveform() {
        let format = Format::new(4).unwrap();
        let mut writer = writer(format, Version::new(1, 4));
        assert!(writer.write(Default::default()).is_err());
    }
}
