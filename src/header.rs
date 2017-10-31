//! Metadata and configuration for las files.
//!
//! Use a `Header` to configure your writer:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Header, Writer};
//! use las::point::Format;
//!
//! let mut header = Header::default();
//! header.version = (1, 4).into();
//! header.point_format = Format::new(6).unwrap();
//! let writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! It's not a perfect [builder pattern](https://aturon.github.io/ownership/builders.html), since
//! it uses public members instead of getters/setters, but you can treat it like a builder. Because
//! there's various combinations of options that may or may not be legal, the legality of the
//! options aren't checked until the header is converted into a `raw::Header`, i.e. we have to
//! commit to the bytes themselves:
//!
//! ```
//! use std::u32;
//! use las::Header;
//! let mut header = Header::default();
//! assert!(header.to_raw().is_ok()); // the default header is legal
//! header.version = (1, 2).into();
//! header.number_of_points = u32::MAX as u64 + 1;
//! assert!(header.to_raw().is_err()); // las 1.2 files can't have this many points
//! header.version = (1, 4).into();
//! assert!(header.to_raw().is_ok()); // but las 1.4 files can
//! ```

use {Bounds, GpsTimeType, Result, Transform, Vector, Version, Vlr, raw};
use chrono::{Date, Datelike, TimeZone, Utc};
use feature::{Evlrs, FileSourceId, GpsStandardTime, SyntheticReturnNumbers};
use point::Format;
use std::collections::HashMap;
use utils::{AsLasStr, FromLasStr};
use uuid::Uuid;

quick_error! {
    /// Header-specific errors.
    #[derive(Clone, Copy, Debug)]
    pub enum Error {
        /// The file signature is not LASF.
        FileSignature(b: [u8; 4]) {
            description("the file signature was not LASF")
            display("the file signature was not LASF: {:?}", b)
        }
        /// The point format is not supported by version.
        Format(version: Version, format: Format) {
            description("format is not supported by version")
            display("format {} is not supported by version {}", format, version)
        }
        /// The offset to point data is too large.
        OffsetToPointDataTooLarge(offset: usize) {
            description("the offset to the point data is too large")
            display("the offset to the point data is too large: {}", offset)
        }
        /// The point data record length is too small for the format.
        PointDataRecordLength(len: u16, format: Format) {
            description("the point data record length is too small for the format")
            display("the point data record length {} is too small for format {}", len, format)
        }
        /// The header size, as computed, is too large.
        TooLarge(len: usize) {
            description("the header is too large to convert to a raw header")
            display("the header is too large to convert to a raw header: {} bytes", len)
        }
        /// Too many extended variable length records.
        TooManyEvlrs(count: usize) {
            description("too many extended variable length records")
            display("too many extended variable length records: {}", count)
        }
        /// Too many points for this version.
        TooManyPoints(n: u64, version: Version) {
            description("too many points for this version")
            display("too many points for this version {}: {}", version, n)
        }
        /// Too many variable length records.
        TooManyVlrs(count: usize) {
            description("too many variable length records")
            display("too many variable length records: {}", count)
        }
        /// The header size, as provided by the raw header, is too small.
        TooSmall(len: u16) {
            description("the header size is too small")
            display("the header size is too small: {}", len)
        }
        /// Wkt is required for this point format.
        WktRequired(format: Format) {
            description("wkt is required for this point format")
            display("wkt is required for point format {}", format)
        }
    }
}

/// Metadata describing the layout, source, and interpretation of the points.
///
/// Headers include *all* las metadata, including (extended) variable length records.
#[derive(Clone, Debug)]
pub struct Header {
    /// A project-wide unique ID for the file.
    pub file_source_id: u16,

    /// The time type for GPS time.
    pub gps_time_type: GpsTimeType,

    /// Are the return numbers sythetic?
    ///
    /// This could be the case, for example, when a composite file is created by combining a First
    /// Return File and a Last Return File.  In this case, first return data will be labeled "1 of
    /// 2" and second return data will be labeled "2 of 2".
    pub has_synthetic_return_numbers: bool,

    /// Optional globally-unique identifier.
    pub guid: Uuid,

    /// The LAS version of this file.
    pub version: Version,

    /// The system that produced this file.
    ///
    /// If hardware, this should be the name of the hardware. Otherwise, maybe describe the
    /// operation performed to create these data?
    pub system_identifier: String,

    /// The software which generated these data.
    pub generating_software: String,

    /// The date these data were collected.
    ///
    /// If the date in the header was crap, this is `None`.
    pub date: Option<Date<Utc>>,

    /// Optional and discouraged padding between the header and the `Vlr`s.
    pub padding: Vec<u8>,

    /// Optional and discouraged padding between the `Vlr`s and the points.
    pub vlr_padding: Vec<u8>,

    /// The `point::Format` of these points.
    pub point_format: Format,

    /// The three `Transform`s used to convert xyz coordinates from floats to signed integers.
    ///
    /// This is how you specify scales and offsets.
    pub transforms: Vector<Transform>,

    /// The bounds of these LAS data.
    pub bounds: Bounds,

    /// The number of points.
    pub number_of_points: u64,

    /// The number of points of each return number.
    pub number_of_points_by_return: HashMap<u8, u64>,

    /// Padding at the end of the points.
    pub end_of_points_padding: Vec<u8>,

    vlrs: Vec<Vlr>,
}

impl Header {
    /// Creates a new header from a raw header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{raw, Header};
    /// let raw_header = raw::Header::default();
    /// let header = Header::new(raw_header).unwrap();
    /// ```
    pub fn new(raw_header: raw::Header) -> Result<Header> {
        let mut point_format = Format::new(raw_header.point_data_format_id)?;
        if raw_header.point_data_record_length > point_format.len() {
            point_format.extra_bytes = raw_header.point_data_record_length - point_format.len();
        } else if raw_header.point_data_record_length < point_format.len() {
            return Err(
                Error::PointDataRecordLength(raw_header.point_data_record_length, point_format)
                    .into(),
            );
        }
        if !is_wkt_bit_set(raw_header.global_encoding) && point_format.is_extended {
            return Err(Error::WktRequired(point_format).into());
        }

        let number_of_points = if raw_header.number_of_point_records > 0 {
            raw_header.number_of_point_records as u64
        } else {
            raw_header
                .large_file
                .map(|f| f.number_of_point_records)
                .unwrap_or(0)
        };
        let number_of_points_by_return =
            if raw_header.number_of_points_by_return.iter().any(|&n| n > 0) {
                number_of_points_hash_map(&raw_header.number_of_points_by_return)
            } else {
                raw_header
                    .large_file
                    .map(|f| number_of_points_hash_map(&f.number_of_points_by_return))
                    .unwrap_or_else(HashMap::new)
            };

        Ok(Header {
            file_source_id: raw_header.file_source_id,
            gps_time_type: raw_header.global_encoding.into(),
            has_synthetic_return_numbers: raw_header.global_encoding & 8 == 8,
            date: Utc.yo_opt(
                raw_header.file_creation_year as i32,
                raw_header.file_creation_day_of_year as u32,
            ).single(),
            generating_software: raw_header
                .generating_software
                .as_ref()
                .as_las_str()?
                .to_string(),
            guid: Uuid::from_bytes(&raw_header.guid).unwrap(),
            padding: raw_header.padding,
            vlr_padding: vec![],
            point_format: point_format,
            number_of_points: number_of_points,
            number_of_points_by_return: number_of_points_by_return,
            system_identifier: raw_header
                .system_identifier
                .as_ref()
                .as_las_str()?
                .to_string(),
            transforms: Vector {
                x: Transform {
                    scale: raw_header.x_scale_factor,
                    offset: raw_header.x_offset,
                },
                y: Transform {
                    scale: raw_header.y_scale_factor,
                    offset: raw_header.y_offset,
                },
                z: Transform {
                    scale: raw_header.z_scale_factor,
                    offset: raw_header.z_offset,
                },
            },
            bounds: Bounds {
                min: Vector {
                    x: raw_header.min_x,
                    y: raw_header.min_y,
                    z: raw_header.min_z,
                },
                max: Vector {
                    x: raw_header.max_x,
                    y: raw_header.max_y,
                    z: raw_header.max_z,
                },
            },
            version: raw_header.version,
            vlrs: vec![],
            end_of_points_padding: vec![],
        })
    }

    /// Adds a variable length record to this header.
    ///
    /// Both extended and regular vlrs are added this way.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Header, Vlr};
    /// let mut header = Header::default();
    /// header.push_vlr(Vlr::default());
    /// header.push_vlr(Vlr { is_extended: true, ..Default::default() });
    /// ```
    pub fn push_vlr(&mut self, vlr: Vlr) {
        self.vlrs.push(vlr);
    }

    /// Returns this header's variable length records.
    ///
    /// If this header's version does not support extended variable length records, all evlrs will
    /// be converted to vlrs *unless* their data are too long.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::u16;
    /// use las::{Header, Vlr};
    /// let mut header = Header::default();
    /// header.version = (1, 4).into();
    /// header.push_vlr(Vlr::default());
    /// header.push_vlr(Vlr { is_extended: true, ..Default::default() });
    /// header.push_vlr(Vlr { data: vec![0; u16::MAX as usize + 1], ..Default::default() });
    /// assert_eq!(1, header.vlrs().len());
    ///
    /// header.version = (1, 2).into();
    /// assert_eq!(2, header.vlrs().len());
    /// ```
    pub fn vlrs(&self) -> Vec<&Vlr> {
        self.filter_vlrs(false)
    }

    /// Returns this header's extended variable length records.
    ///
    /// If this header's version supports extended variable length records, all too-long regular
    /// vlrs will be converted to extended.
    ///
    /// If this header's version does not support extended variable length records, this will only
    /// return too-long vlrs.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::u16;
    /// use las::{Header, Vlr};
    /// let mut header = Header::default();
    /// header.version = (1, 4).into();
    /// header.push_vlr(Vlr::default());
    /// header.push_vlr(Vlr { is_extended: true, ..Default::default() });
    /// header.push_vlr(Vlr { data: vec![0; u16::MAX as usize + 1], ..Default::default() });
    /// assert_eq!(2, header.evlrs().len());
    ///
    /// header.version = (1, 2).into();
    /// assert_eq!(1, header.evlrs().len());
    /// ```
    pub fn evlrs(&self) -> Vec<&Vlr> {
        self.filter_vlrs(true)
    }

    /// Returns the position of the first byte after the point records end.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let mut header = Header::default();
    /// assert_eq!(227, header.offset_to_end_of_points().unwrap());
    /// header.number_of_points = 1;
    /// assert_eq!(247, header.offset_to_end_of_points().unwrap());
    /// header.point_format.extra_bytes = 1;
    /// assert_eq!(248, header.offset_to_end_of_points().unwrap());
    /// ```
    pub fn offset_to_end_of_points(&self) -> Result<u64> {
        self.offset_to_point_data().map(|n| {
            u64::from(n) + self.point_data_len()
        })
    }

    /// Converts this header into a raw header.
    ///
    /// This does all the necessary version support checking to verify that we are following The
    /// Rules.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Header, GpsTimeType};
    /// let mut header = Header::default();
    /// let raw_header = header.to_raw().unwrap();
    /// header.gps_time_type = GpsTimeType::Standard;
    /// header.version = (1, 0).into();
    /// assert!(header.to_raw().is_err());
    /// ```
    pub fn to_raw(&self) -> Result<raw::Header> {
        if self.file_source_id != 0 {
            self.version.verify_support_for::<FileSourceId>()?;
        }
        if !self.version.supports_point_format(self.point_format) {
            return Err(Error::Format(self.version, self.point_format).into());
        }
        // TODO check waveforms
        Ok(raw::Header {
            file_signature: raw::LASF,
            file_source_id: self.file_source_id,
            global_encoding: self.global_encoding()?,
            guid: *self.guid.as_bytes(),
            version: self.version,
            system_identifier: self.system_identifier()?,
            generating_software: self.generating_software()?,
            file_creation_day_of_year: self.date.map_or(0, |d| d.ordinal() as u16),
            file_creation_year: self.date.map_or(0, |d| d.year() as u16),
            header_size: self.header_size()?,
            offset_to_point_data: self.offset_to_point_data()?,
            number_of_variable_length_records: self.number_of_variable_length_records()?,
            point_data_format_id: self.point_format.to_u8()?,
            point_data_record_length: self.point_format.len(),
            number_of_point_records: self.number_of_points()?,
            number_of_points_by_return: self.number_of_points_by_return()?,
            x_scale_factor: self.transforms.x.scale,
            y_scale_factor: self.transforms.y.scale,
            z_scale_factor: self.transforms.z.scale,
            x_offset: self.transforms.x.offset,
            y_offset: self.transforms.y.offset,
            z_offset: self.transforms.z.offset,
            max_x: self.bounds.max.x,
            min_x: self.bounds.min.x,
            max_y: self.bounds.max.y,
            min_y: self.bounds.min.y,
            max_z: self.bounds.max.z,
            min_z: self.bounds.min.z,
            start_of_waveform_data_packet_record: None,
            evlr: self.evlr()?,
            large_file: self.large_file()?,
            padding: self.padding.clone(),
        })
    }

    fn global_encoding(&self) -> Result<u16> {
        if self.gps_time_type.is_standard() {
            self.version.verify_support_for::<GpsStandardTime>()?;
        }
        let mut bits = self.gps_time_type.into();
        if self.has_synthetic_return_numbers {
            self.version.verify_support_for::<SyntheticReturnNumbers>()?;
            bits |= 8;
        }
        if self.point_format.is_extended {
            bits |= 16;
        }
        Ok(bits)
    }

    fn system_identifier(&self) -> Result<[u8; 32]> {
        let mut system_identifier = [0; 32];
        system_identifier.as_mut().from_las_str(
            &self.system_identifier,
        )?;
        Ok(system_identifier)
    }

    fn generating_software(&self) -> Result<[u8; 32]> {
        let mut generating_software = [0; 32];
        generating_software.as_mut().from_las_str(
            &self.generating_software,
        )?;
        Ok(generating_software)
    }

    fn header_size(&self) -> Result<u16> {
        use std::u16;

        let header_size = self.version.header_size() as usize + self.padding.len();
        if header_size > u16::MAX as usize {
            Err(Error::TooLarge(header_size).into())
        } else {
            Ok(header_size as u16)
        }
    }

    fn offset_to_point_data(&self) -> Result<u32> {
        use std::u32;

        let vlr_len = self.vlrs().iter().fold(0, |acc, vlr| acc + vlr.len());
        let offset = self.header_size()? as usize + vlr_len + self.vlr_padding.len();
        if offset > u32::MAX as usize {
            Err(Error::OffsetToPointDataTooLarge(offset).into())
        } else {
            Ok(offset as u32)
        }
    }

    fn number_of_variable_length_records(&self) -> Result<u32> {
        use std::u32;

        let n = self.vlrs().len();
        if n > u32::MAX as usize {
            Err(Error::TooManyVlrs(n).into())
        } else {
            Ok(n as u32)
        }
    }

    fn number_of_points(&self) -> Result<u32> {
        use std::u32;
        use feature::LargeFiles;

        if self.number_of_points > u32::MAX as u64 {
            if self.version.supports::<LargeFiles>() {
                Ok(0)
            } else {
                Err(
                    Error::TooManyPoints(self.number_of_points, self.version).into(),
                )
            }
        } else {
            Ok(self.number_of_points as u32)
        }
    }

    fn number_of_points_by_return(&self) -> Result<[u32; 5]> {
        use std::u32;
        use feature::LargeFiles;

        let mut number_of_points_by_return = [0; 5];
        for (&i, &n) in &self.number_of_points_by_return {
            if i > 5 {
                if !self.version.supports::<LargeFiles>() {
                    return Err(::point::Error::ReturnNumber(i, Some(self.version)).into());
                }
            } else if i > 0 {
                if n > u32::MAX as u64 {
                    if !self.version.supports::<LargeFiles>() {
                        return Err(Error::TooManyPoints(n, self.version).into());
                    }
                } else {
                    number_of_points_by_return[i as usize - 1] = n as u32;
                }
            }
        }
        Ok(number_of_points_by_return)
    }

    fn evlr(&self) -> Result<Option<raw::header::Evlr>> {
        use std::u32;

        let n = self.evlrs().len();
        if n > 0 {
            self.version.verify_support_for::<Evlrs>()?;
        }
        if n == 0 {
            Ok(None)
        } else if n > u32::MAX as usize {
            Err(Error::TooManyEvlrs(n).into())
        } else {
            let start_of_first_evlr = u64::from(self.offset_to_point_data()?) +
                self.point_data_len();
            Ok(Some(raw::header::Evlr {
                start_of_first_evlr: start_of_first_evlr,
                number_of_evlrs: n as u32,
            }))
        }
    }

    fn large_file(&self) -> Result<Option<raw::header::LargeFile>> {
        let mut number_of_points_by_return = [0; 15];
        for (&i, &n) in &self.number_of_points_by_return {
            if i > 15 {
                return Err(::point::Error::ReturnNumber(i, Some(self.version)).into());
            } else if i > 0 {
                number_of_points_by_return[i as usize - 1] = n;
            }
        }
        Ok(Some(raw::header::LargeFile {
            number_of_point_records: self.number_of_points,
            number_of_points_by_return: number_of_points_by_return,
        }))
    }

    fn point_data_len(&self) -> u64 {
        u64::from(self.number_of_points) * u64::from(self.point_format.len())
    }

    fn filter_vlrs(&self, extended: bool) -> Vec<&Vlr> {
        use std::u16;
        use feature::Evlrs;

        self.vlrs
            .iter()
            .filter(|vlr| {
                (vlr.len() > u16::MAX as usize ||
                     (self.version.supports::<Evlrs>() && vlr.is_extended)) ==
                    extended
            })
            .collect()
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            file_source_id: 0,
            gps_time_type: GpsTimeType::Week,
            has_synthetic_return_numbers: false,
            bounds: Default::default(),
            date: Some(Utc::today()),
            generating_software: format!("las-rs {}", env!("CARGO_PKG_VERSION")),
            guid: Default::default(),
            number_of_points: 0,
            number_of_points_by_return: HashMap::new(),
            padding: Vec::new(),
            vlr_padding: Vec::new(),
            point_format: Default::default(),
            system_identifier: "las-rs".to_string(),
            transforms: Default::default(),
            version: Default::default(),
            vlrs: Vec::new(),
            end_of_points_padding: Vec::new(),
        }
    }
}

fn number_of_points_hash_map<T: Copy + Into<u64>>(slice: &[T]) -> HashMap<u8, u64> {
    use std::u8;
    assert!(slice.len() < u8::MAX as usize);
    slice
        .iter()
        .enumerate()
        .filter_map(|(i, &n)| if n.into() > 0 {
            Some((i as u8 + 1, n.into()))
        } else {
            None
        })
        .collect()
}

fn is_wkt_bit_set(n: u16) -> bool {
    n & 16 == 16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_data_record_length_too_small() {
        let raw_header = raw::Header {
            point_data_record_length: 19,
            ..Default::default()
        };
        assert!(Header::new(raw_header).is_err());
    }

    #[test]
    fn extra_bytes() {
        let raw_header = raw::Header {
            point_data_record_length: 21,
            ..Default::default()
        };
        let header = Header::new(raw_header).unwrap();
        assert_eq!(1, header.point_format.extra_bytes);
    }

    #[test]
    fn no_day_no_date() {
        let raw_header = raw::Header {
            file_creation_day_of_year: 0,
            ..Default::default()
        };
        let header = Header::new(raw_header).unwrap();
        assert!(header.date.is_none());
    }

    #[test]
    fn no_year_no_date() {
        let raw_header = raw::Header {
            file_creation_year: 0,
            ..Default::default()
        };
        let header = Header::new(raw_header).unwrap();
        assert!(header.date.is_none());
    }

    #[test]
    fn number_of_points_by_return_zero_return_number() {
        let mut header = Header::default();
        header.number_of_points_by_return.insert(0, 1);
        assert_eq!([0; 5], header.to_raw().unwrap().number_of_points_by_return);
    }

    #[test]
    fn number_of_points_by_return_las_1_2() {
        let mut header = Header::default();
        header.version = (1, 2).into();
        for i in 1..6 {
            header.number_of_points_by_return.insert(i, 42);
        }
        assert_eq!([42; 5], header.to_raw().unwrap().number_of_points_by_return);
    }

    #[test]
    fn number_of_points_by_return_las_1_2_return_6() {
        let mut header = Header::default();
        header.version = (1, 2).into();
        header.number_of_points_by_return.insert(6, 1);
        assert!(header.to_raw().is_err());
    }

    #[test]
    fn header_too_large() {
        use std::u16;
        let header = Header {
            padding: vec![0; u16::MAX as usize - 226],
            version: (1, 2).into(),
            ..Default::default()
        };
        assert!(header.to_raw().is_err());
    }

    #[test]
    fn offset_to_point_data_too_large() {
        use std::u32;
        let header = Header {
            vlr_padding: vec![0; u32::MAX as usize - 226],
            version: (1, 2).into(),
            ..Default::default()
        };
        assert!(header.to_raw().is_err());
    }

    #[test]
    fn synchronize_legacy_fields() {
        let mut header = Header {
            version: (1, 4).into(),
            number_of_points: 42,
            ..Default::default()
        };
        header.number_of_points_by_return.insert(2, 42);
        let raw_header = header.to_raw().unwrap();
        assert_eq!(42, raw_header.number_of_point_records);
        assert_eq!([0, 42, 0, 0, 0], raw_header.number_of_points_by_return);
        assert_eq!(42, raw_header.large_file.unwrap().number_of_point_records);
        assert_eq!(
            [0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            raw_header.large_file.unwrap().number_of_points_by_return
        );
    }

    #[test]
    fn zero_legacy_fields_when_too_large() {
        use std::u32;

        let mut header = Header {
            version: (1, 4).into(),
            number_of_points: u32::MAX as u64 + 1,
            ..Default::default()
        };
        header.number_of_points_by_return.insert(6, 42);
        let raw_header = header.to_raw().unwrap();
        assert_eq!(0, raw_header.number_of_point_records);
        assert_eq!(
            u32::MAX as u64 + 1,
            raw_header.large_file.unwrap().number_of_point_records
        );
        assert_eq!([0; 5], raw_header.number_of_points_by_return);
        assert_eq!(
            [0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            raw_header.large_file.unwrap().number_of_points_by_return
        );
    }

    #[test]
    fn prefer_legacy_fields() {
        let mut raw_header = raw::Header::default();
        raw_header.version = (1, 4).into();
        raw_header.number_of_point_records = 42;
        raw_header.number_of_points_by_return[0] = 42;
        let mut large_file = raw::header::LargeFile::default();
        large_file.number_of_point_records = 43;
        large_file.number_of_points_by_return[0] = 43;
        raw_header.large_file = Some(large_file);
        let header = Header::new(raw_header).unwrap();
        assert_eq!(42, header.number_of_points);
        assert_eq!(42, header.number_of_points_by_return[&1]);
    }

    #[test]
    fn number_of_points_large() {
        use std::u32;

        let mut header = Header::default();
        header.version = (1, 2).into();
        header.number_of_points = u32::MAX as u64 + 1;
        assert!(header.to_raw().is_err());
        header.version = (1, 4).into();
        let raw_header = header.to_raw().unwrap();
        assert_eq!(0, raw_header.number_of_point_records);
        assert_eq!(
            u32::MAX as u64 + 1,
            raw_header.large_file.unwrap().number_of_point_records
        );
    }

    #[test]
    fn number_of_points_by_return_large() {
        use std::u32;

        let mut header = Header::default();
        header.version = (1, 2).into();
        header.number_of_points_by_return.insert(
            1,
            u32::MAX as u64 + 1,
        );
        assert!(header.to_raw().is_err());
        header.version = (1, 4).into();
        let raw_header = header.to_raw().unwrap();
        assert_eq!(0, raw_header.number_of_points_by_return[0]);
        assert_eq!(
            u32::MAX as u64 + 1,
            raw_header.large_file.unwrap().number_of_points_by_return[0]
        );
    }

    #[test]
    fn wkt_bit() {
        let mut header = Header::default();
        header.version = (1, 4).into();
        let raw_header = header.to_raw().unwrap();
        assert_eq!(0, raw_header.global_encoding);
        header.point_format = Format::new(6).unwrap();
        let raw_header = header.to_raw().unwrap();
        assert_eq!(0b10000, raw_header.global_encoding);
    }

    macro_rules! point_format {
        ($name:ident, $format:expr) => {
            mod $name {
                use super::*;

                #[test]
                fn wkt() {
                    let raw_header = raw::Header {
                        version: (1, 4).into(),
                        point_data_format_id: $format,
                        point_data_record_length: Format::new($format).unwrap().len(),
                        ..Default::default()
                    };
                    if $format < 6 {
                        Header::new(raw_header).unwrap();
                    } else {
                        assert!(Header::new(raw_header).is_err());
                    }
                }
            }
        }
    }

    point_format!(point_format_0, 0);
    point_format!(point_format_1, 1);
    point_format!(point_format_2, 2);
    point_format!(point_format_3, 3);
    point_format!(point_format_4, 4);
    point_format!(point_format_5, 5);
    point_format!(point_format_6, 6);
    point_format!(point_format_7, 7);
    point_format!(point_format_8, 8);
    point_format!(point_format_9, 9);
    point_format!(point_format_10, 10);
}
