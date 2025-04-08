//! A [Header] describes the configuration and properties of las data.
//!
//! # Reading
//!
//! A [Reader](crate::Reader) uses a [Header] to expose metadata:
//!
//! ```
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let header = reader.header();
//! println!("The file has {} points.", header.number_of_points());
//! ```
//!
//! # Writing
//!
//! A [Writer](crate::Writer) uses a [Header] to configure how it will write
//! points.  To create a las file, you can use a [Header] from another file, use
//! the default [Header], or create one with a [Builder]:
//!
//! The default configuration writes to a [Cursor](std::io::Cursor):
//!
//! ```
//! use las::Writer;
//!
//! let writer = Writer::default();
//! ```
//!
//! You can copy configuration from an existing file:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Reader};
//!
//! let header = Reader::from_path("tests/data/autzen.las").unwrap().header().clone();
//! let writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! Use a [Builder] to set your own configuration:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Builder};
//!
//! let mut builder = Builder::from((1, 4));
//! builder.system_identifier = "Synthetic points".to_string();
//! let header = builder.into_header().unwrap();
//! let writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! # Into raw bytes
//!
//! A [Header] has a method to turn it into a [raw::Header], which maps directly
//! onto the bytes of the las spec:
//!
//! ```
//! use las::Header;
//! let header = Header::default();
//! let raw_header = header.into_raw().unwrap();
//! assert_eq!(b"LASF", &raw_header.file_signature);
//! ```

pub use self::builder::Builder;
use crate::{
    point::Format, raw, utils::FromLasStr, Bounds, Error, GpsTimeType, Point, Result, Transform,
    Vector, Version, Vlr,
};
use chrono::{Datelike, NaiveDate, Utc};
use std::{collections::HashMap, io::Write, iter::Chain, slice::Iter};
use thiserror::Error;
use uuid::Uuid;

mod builder;

/// Metadata describing the layout, source, and interpretation of the points.
///
/// Headers include *all* las metadata, including regular and extended variable length records and
/// any file padding (e.g. extra bytes after the header).
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    bounds: Bounds,
    date: Option<NaiveDate>,
    evlrs: Vec<Vlr>,
    file_source_id: u16,
    generating_software: String,
    gps_time_type: GpsTimeType,
    guid: Uuid,
    has_synthetic_return_numbers: bool,
    has_wkt_crs: bool,
    number_of_points: u64,
    number_of_points_by_return: HashMap<u8, u64>,
    padding: Vec<u8>,
    point_format: Format,
    point_padding: Vec<u8>,
    start_of_first_evlr: Option<u64>,
    system_identifier: String,
    transforms: Vector<Transform>,
    version: Version,
    vlr_padding: Vec<u8>,
    pub(crate) vlrs: Vec<Vlr>,
}

/// An iterator over a header's variable length records.
///
/// Get this iterator via [Header::vlrs] or [Header::evlrs].
#[derive(Debug)]
pub struct Vlrs<'a>(Chain<Iter<'a, Vlr>, Iter<'a, Vlr>>);

impl Header {
    /// Creates a new header from a raw header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{raw, Header};
    /// let raw_header = raw::Header::default();
    /// let header = Header::from_raw(raw_header).unwrap();
    /// ```
    pub fn from_raw(raw_header: raw::Header) -> Result<Header> {
        Builder::new(raw_header).and_then(|b| b.into_header())
    }

    /// Clears this header's point counts and bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Header, Point, Bounds};
    /// let mut header = Header::default();
    /// header.add_point(&Point { return_number: 1, ..Default::default() });
    /// assert_eq!(1, header.number_of_points());
    /// assert_eq!(1, header.number_of_points_by_return(1).unwrap());
    /// header.clear();
    /// assert_eq!(0, header.number_of_points());
    /// assert_eq!(None, header.number_of_points_by_return(1));
    /// assert_eq!(Bounds::default(), header.bounds());
    /// ```
    pub fn clear(&mut self) {
        self.number_of_points = 0;
        self.number_of_points_by_return = Default::default();
        self.bounds = Default::default();
    }

    /// Adds a point to this header, incrementing the point counts and growing the bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let mut header = Header::default();
    /// header.add_point(&Default::default());
    /// assert_eq!(1, header.number_of_points());
    /// ```
    pub fn add_point(&mut self, point: &Point) {
        self.number_of_points += 1;
        if point.return_number > 0 {
            let entry = self
                .number_of_points_by_return
                .entry(point.return_number)
                .or_insert(0);
            *entry += 1;
        }
        self.bounds.grow(point);
    }

    /// Returns this header's file source id.
    ///
    /// For airborne data, this is often the flight line number.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert_eq!(0, Header::default().file_source_id());
    /// ```
    pub fn file_source_id(&self) -> u16 {
        self.file_source_id
    }

    /// Returns the gps time type.
    ///
    /// This affects what the gps time values on points means.
    /// [GpsTimeType::Week] means that the time values are seconds from the
    /// start of the week. [GpsTimeType::Standard] means that the time values
    /// are standard GPS time (satellite gps time) minus 10e9.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{GpsTimeType, Header};
    /// assert_eq!(GpsTimeType::Week, Header::default().gps_time_type());
    /// ```
    pub fn gps_time_type(&self) -> GpsTimeType {
        self.gps_time_type
    }

    /// Returns true if the return numbers on the point data records have been synthetically
    /// generated.
    ///
    /// Only supported in later las versions.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(!Header::default().has_synthetic_return_numbers());
    pub fn has_synthetic_return_numbers(&self) -> bool {
        self.has_synthetic_return_numbers
    }

    /// Returns true if the coordinate reference system is Well Known Text (WKT).
    ///
    /// Only supported in las 1.4.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(!Header::default().has_wkt_crs());
    pub fn has_wkt_crs(&self) -> bool {
        self.has_wkt_crs
    }

    /// Returns this header's guid.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let guid = Header::default().guid();
    /// ```
    pub fn guid(&self) -> Uuid {
        self.guid
    }

    /// Returns this header's version.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Header, Version};
    /// assert_eq!(Version::new(1, 2), Header::default().version());
    /// ```
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns this header's system identifier.
    ///
    /// Describes the source of the data, whether it is a sensor or a processing operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// println!("{}", Header::default().system_identifier());
    /// ```
    pub fn system_identifier(&self) -> &str {
        &self.system_identifier
    }

    /// Returns this header's generating software.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(Header::default().generating_software().starts_with("las-rs"));
    /// ```
    pub fn generating_software(&self) -> &str {
        &self.generating_software
    }

    /// Returns this header's file creation date.
    ///
    /// Can be `None`, which is against spec but happens with files in the wild.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let date = Header::default().date().unwrap();
    /// ```
    pub fn date(&self) -> Option<NaiveDate> {
        self.date
    }

    /// Returns this header's padding.
    ///
    /// These are bytes that are after the header but before the vlr. Not recommended to use.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(Header::default().padding().is_empty());
    /// ```
    pub fn padding(&self) -> &Vec<u8> {
        &self.padding
    }

    /// Returns this header's point format.
    ///
    /// Point formats are used to describe the attributes and extra bytes of each point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let header = Header::default();
    /// assert_eq!(0, header.point_format().to_u8().unwrap());
    /// ```
    pub fn point_format(&self) -> &Format {
        &self.point_format
    }

    pub(crate) fn point_format_mut(&mut self) -> &mut Format {
        &mut self.point_format
    }

    /// Returns this header's transforms.
    ///
    /// The transforms are the scales and offsets used to convert floating point numbers to `i16`.
    /// Las data stores point coordinates as `i16`s internally.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let header = Header::default();
    /// let transforms = header.transforms();
    /// assert_eq!(0.001, transforms.x.scale);
    /// ```
    pub fn transforms(&self) -> &Vector<Transform> {
        &self.transforms
    }

    /// Returns the bounds of this header.
    ///
    /// The bounds describe the min and max values in each dimension.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let bounds = Header::default().bounds();
    /// ```
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Returns this header's number of points.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let header = Header::default();
    /// assert_eq!(0, header.number_of_points());
    /// ```
    pub fn number_of_points(&self) -> u64 {
        self.number_of_points
    }

    /// Returns this header's number of points for a given return number.
    ///
    /// Note that return numbers are 1-indexed.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let header = Header::default();
    /// assert_eq!(None, header.number_of_points_by_return(1));
    /// ```
    pub fn number_of_points_by_return(&self, n: u8) -> Option<u64> {
        self.number_of_points_by_return.get(&n).copied()
    }

    /// Returns a reference to this header's vlr padding.
    ///
    /// These are bytes after the vlrs but before the points. Again, not recommended for use.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(Header::default().vlr_padding().is_empty());
    /// ```
    pub fn vlr_padding(&self) -> &Vec<u8> {
        &self.vlr_padding
    }

    /// Returns a reference to this header's point padding.
    ///
    /// These are the bytes after the points but before eof/any evlrs. Not recommended.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// assert!(Header::default().point_padding().is_empty());
    /// ```
    pub fn point_padding(&self) -> &Vec<u8> {
        &self.point_padding
    }

    /// Returns a reference to this header's vlrs.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Vlr, Builder};
    /// let mut builder = Builder::default();
    /// builder.vlrs.push(Vlr::default());
    /// let header = builder.into_header().unwrap();
    /// assert_eq!(1, header.vlrs().len());
    /// ```
    pub fn vlrs(&self) -> &Vec<Vlr> {
        &self.vlrs
    }

    /// Returns a reference to header's extended variable length records.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Vlr, Builder};
    /// let mut builder = Builder::from((1, 4));
    /// builder.evlrs.push(Vlr::default());
    /// let header = builder.into_header().unwrap();
    /// assert_eq!(1, header.evlrs().len());
    /// ```
    pub fn evlrs(&self) -> &Vec<Vlr> {
        &self.evlrs
    }

    /// Returns an iterator over all this header's vlrs, both extended and regular.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Vlr, Builder};
    /// let mut builder = Builder::from((1, 4));
    /// builder.vlrs.push(Vlr::default());
    /// builder.evlrs.push(Vlr::default());
    /// let header = builder.into_header().unwrap();
    /// assert_eq!(2, header.all_vlrs().count());
    pub fn all_vlrs(&self) -> Vlrs<'_> {
        Vlrs(self.vlrs.iter().chain(&self.evlrs))
    }

    /// Converts this header into a raw header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let raw_header = Header::default().into_raw().unwrap();
    /// ```
    pub fn into_raw(self) -> Result<raw::Header> {
        // Scale the bounding box properly
        let bounds = self.bounds.adapt(&self.transforms)?;
        Ok(raw::Header {
            file_signature: raw::LASF,
            file_source_id: self.file_source_id,
            global_encoding: self.global_encoding(),
            guid: *self.guid.as_bytes(),
            version: self.version,
            system_identifier: self.system_identifier_raw()?,
            generating_software: self.generating_software_raw()?,
            file_creation_day_of_year: self.date.map_or(0, |d| d.ordinal() as u16),
            file_creation_year: self.date.map_or(0, |d| d.year() as u16),
            header_size: self.header_size()?,
            offset_to_point_data: self.offset_to_point_data()?,
            number_of_variable_length_records: self.number_of_variable_length_records()?,
            point_data_record_format: self.point_format.to_writable_u8()?,
            point_data_record_length: self.point_format.len(),
            number_of_point_records: self.number_of_points_raw()?,
            number_of_points_by_return: self.number_of_points_by_return_raw()?,
            x_scale_factor: self.transforms.x.scale,
            y_scale_factor: self.transforms.y.scale,
            z_scale_factor: self.transforms.z.scale,
            x_offset: self.transforms.x.offset,
            y_offset: self.transforms.y.offset,
            z_offset: self.transforms.z.offset,
            max_x: bounds.max.x,
            min_x: bounds.min.x,
            max_y: bounds.max.y,
            min_y: bounds.min.y,
            max_z: bounds.max.z,
            min_z: bounds.min.z,
            // FIXME waveforms
            start_of_waveform_data_packet_record: None,
            evlr: self.evlr()?,
            large_file: self.large_file()?,
            padding: self.padding,
        })
    }

    /// Writes this header to a [Write].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::Header;
    ///
    /// let header = Header::default();
    /// let cursor = Cursor::new(Vec::new());
    /// header.write_to(cursor).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W) -> Result<()> {
        self.clone()
            .into_raw()
            .and_then(|raw_header| raw_header.write_to(&mut write))?;
        for vlr in self.vlrs() {
            (*vlr)
                .clone()
                .into_raw(false)
                .and_then(|raw_vlr| raw_vlr.write_to(&mut write))?;
        }
        if !self.vlr_padding().is_empty() {
            write.write_all(self.vlr_padding())?;
        }
        Ok(())
    }

    pub(crate) fn set_start_of_first_evlr(&mut self, start_of_first_evlr: u64) {
        self.start_of_first_evlr = Some(start_of_first_evlr);
    }

    fn global_encoding(&self) -> u16 {
        let mut bits = self.gps_time_type.into();
        if self.has_synthetic_return_numbers {
            bits |= 8;
        }
        if self.has_wkt_crs || self.point_format.is_extended {
            bits |= 16;
        }
        bits
    }

    fn system_identifier_raw(&self) -> Result<[u8; 32]> {
        let mut system_identifier = [0; 32];
        system_identifier
            .as_mut()
            .from_las_str(&self.system_identifier)?;
        Ok(system_identifier)
    }

    fn generating_software_raw(&self) -> Result<[u8; 32]> {
        let mut generating_software = [0; 32];
        generating_software
            .as_mut()
            .from_las_str(&self.generating_software)?;
        Ok(generating_software)
    }

    fn header_size(&self) -> Result<u16> {
        let header_size = self.version.header_size() as usize + self.padding.len();
        if header_size > u16::MAX as usize {
            Err(Error::HeaderTooLarge(header_size))
        } else {
            Ok(header_size as u16)
        }
    }

    fn offset_to_point_data(&self) -> Result<u32> {
        let vlr_len = self.vlrs.iter().fold(0, |acc, vlr| acc + vlr.len(false));
        let offset = self.header_size()? as usize + vlr_len + self.vlr_padding.len();
        if offset > u32::MAX as usize {
            Err(Error::OffsetToPointDataTooLarge(offset))
        } else {
            Ok(offset as u32)
        }
    }

    fn number_of_variable_length_records(&self) -> Result<u32> {
        let n = self.vlrs().len();
        if n > u32::MAX as usize {
            Err(Error::TooManyVlrs(n))
        } else {
            Ok(n as u32)
        }
    }

    fn number_of_points_raw(&self) -> Result<u32> {
        use crate::feature::LargeFiles;

        if self.number_of_points > u64::from(u32::MAX) {
            if self.version.supports::<LargeFiles>() {
                Ok(0)
            } else {
                Err(Error::TooManyPoints {
                    n: self.number_of_points,
                    version: self.version,
                })
            }
        } else {
            Ok(self.number_of_points as u32)
        }
    }

    fn number_of_points_by_return_raw(&self) -> Result<[u32; 5]> {
        use crate::feature::LargeFiles;

        let mut number_of_points_by_return = [0; 5];
        for (&i, &n) in &self.number_of_points_by_return {
            if i > 5 {
                if !self.version.supports::<LargeFiles>() {
                    return Err(Error::ReturnNumber {
                        return_number: i,
                        version: Some(self.version),
                    });
                }
            } else if i > 0 {
                if n > u64::from(u32::MAX) {
                    if !self.version.supports::<LargeFiles>() {
                        return Err(Error::TooManyPoints {
                            n,
                            version: self.version,
                        });
                    }
                } else {
                    number_of_points_by_return[i as usize - 1] = n as u32;
                }
            }
        }
        Ok(number_of_points_by_return)
    }

    fn evlr(&self) -> Result<Option<raw::header::Evlr>> {
        let n = self.evlrs.len();
        if n == 0 {
            Ok(None)
        } else if n > u32::MAX as usize {
            Err(Error::TooManyEvlrs(n))
        } else {
            let start_of_first_evlr = if let Some(start_of_fist_evlr) = self.start_of_first_evlr {
                start_of_fist_evlr
            } else {
                self.point_data_len()
                    + self.point_padding.len() as u64
                    + u64::from(self.offset_to_point_data()?)
            };
            Ok(Some(raw::header::Evlr {
                start_of_first_evlr,
                number_of_evlrs: n as u32,
            }))
        }
    }

    fn large_file(&self) -> Result<Option<raw::header::LargeFile>> {
        let mut number_of_points_by_return = [0; 15];
        for (&i, &n) in &self.number_of_points_by_return {
            if i > 15 {
                return Err(Error::ReturnNumber {
                    return_number: i,
                    version: Some(self.version),
                });
            } else if i > 0 {
                number_of_points_by_return[i as usize - 1] = n;
            }
        }
        Ok(Some(raw::header::LargeFile {
            number_of_point_records: self.number_of_points,
            number_of_points_by_return,
        }))
    }

    fn point_data_len(&self) -> u64 {
        self.number_of_points * u64::from(self.point_format.len())
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            bounds: Default::default(),
            date: Some(Utc::now().date_naive()),
            evlrs: Vec::new(),
            file_source_id: 0,
            generating_software: format!("las-rs {}", env!("CARGO_PKG_VERSION")),
            gps_time_type: GpsTimeType::Week,
            guid: Default::default(),
            has_synthetic_return_numbers: false,
            has_wkt_crs: false,
            number_of_points: 0,
            number_of_points_by_return: HashMap::new(),
            padding: Vec::new(),
            point_format: Default::default(),
            point_padding: Vec::new(),
            start_of_first_evlr: None,
            system_identifier: "las-rs".to_string(),
            transforms: Default::default(),
            version: Default::default(),
            vlr_padding: Vec::new(),
            vlrs: Vec::new(),
        }
    }
}

impl<V: Into<Version>> From<V> for Header {
    fn from(version: V) -> Header {
        Builder::from(version)
            .into_header()
            .expect("Default builder could not be converted into a header")
    }
}

impl<'a> Iterator for Vlrs<'a> {
    type Item = &'a Vlr;
    fn next(&mut self) -> Option<&'a Vlr> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_of_points_by_return_zero_return_number() {
        let mut header = Header::default();
        header.add_point(&Default::default());
        assert_eq!(
            [0; 5],
            header.into_raw().unwrap().number_of_points_by_return
        );
    }

    #[test]
    fn number_of_points_by_return_las_1_2() {
        let mut header = Header::from((1, 2));
        for i in 1..6 {
            let point = Point {
                return_number: i,
                ..Default::default()
            };
            for _ in 0..42 {
                header.add_point(&point);
            }
        }
        assert_eq!(
            [42; 5],
            header.into_raw().unwrap().number_of_points_by_return
        );
    }

    #[test]
    fn number_of_points_by_return_las_1_2_return_6() {
        let mut header = Header::from((1, 2));
        header.add_point(&Point {
            return_number: 6,
            ..Default::default()
        });
        assert!(header.into_raw().is_err());
    }

    #[test]
    fn synchronize_legacy_fields() {
        let mut header = Header::from((1, 4));
        let point = Point {
            return_number: 2,
            ..Default::default()
        };
        for _ in 0..42 {
            header.add_point(&point);
        }
        let raw_header = header.into_raw().unwrap();
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
        let mut header = Header::from((1, 4));
        header.number_of_points = u64::from(u32::MAX) + 1;
        let _ = header.number_of_points_by_return.insert(6, 42);
        let raw_header = header.into_raw().unwrap();
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
        let mut raw_header = raw::Header {
            version: (1, 4).into(),
            number_of_point_records: 42,
            number_of_points_by_return: [42, 0, 0, 0, 0],
            ..Default::default()
        };
        let large_file = raw::header::LargeFile {
            number_of_point_records: 43,
            number_of_points_by_return: [43, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        raw_header.large_file = Some(large_file);
        let header = Header::from_raw(raw_header).unwrap();
        assert_eq!(42, header.number_of_points());
        assert_eq!(42, header.number_of_points_by_return(1).unwrap());
    }

    #[test]
    fn number_of_points_large() {
        let mut header = Header::from((1, 2));
        header.number_of_points = u32::MAX as u64 + 1;
        assert!(header.into_raw().is_err());

        let mut header = Header::from((1, 4));
        header.number_of_points = u32::MAX as u64 + 1;
        let raw_header = header.into_raw().unwrap();
        assert_eq!(0, raw_header.number_of_point_records);
        assert_eq!(
            u32::MAX as u64 + 1,
            raw_header.large_file.unwrap().number_of_point_records
        );

        let header = Header::from_raw(raw_header).unwrap();
        assert_eq!(u32::MAX as u64 + 1, header.number_of_points);
    }

    #[test]
    fn number_of_points_by_return_large() {
        let mut header = Header::from((1, 2));
        let _ = header
            .number_of_points_by_return
            .insert(1, u32::MAX as u64 + 1);
        assert!(header.into_raw().is_err());

        let mut header = Header::from((1, 4));
        let _ = header
            .number_of_points_by_return
            .insert(1, u32::MAX as u64 + 1);
        let raw_header = header.into_raw().unwrap();
        assert_eq!(0, raw_header.number_of_points_by_return[0]);
        assert_eq!(
            u32::MAX as u64 + 1,
            raw_header.large_file.unwrap().number_of_points_by_return[0]
        );
    }

    #[test]
    fn wkt_bit() {
        let mut header = Header::from((1, 4));
        let raw_header = header.clone().into_raw().unwrap();
        assert_eq!(0, raw_header.global_encoding);
        header.has_wkt_crs = true;
        let raw_header = header.clone().into_raw().unwrap();
        assert_eq!(16, raw_header.global_encoding);
        header.has_wkt_crs = false;
        header.point_format = Format::new(6).unwrap();
        let raw_header = header.into_raw().unwrap();
        assert_eq!(16, raw_header.global_encoding);
    }

    #[test]
    fn header_too_large() {
        let builder = Builder::new(raw::Header {
            padding: vec![0; u16::MAX as usize - 226],
            version: (1, 2).into(),
            ..Default::default()
        })
        .unwrap();
        assert!(builder.into_header().unwrap().into_raw().is_err());
    }

    #[test]
    fn offset_to_point_data_too_large() {
        let mut builder = Builder::from((1, 2));
        builder.vlr_padding = vec![0; u32::MAX as usize - 226];
        assert!(builder.into_header().unwrap().into_raw().is_err());
    }
}
