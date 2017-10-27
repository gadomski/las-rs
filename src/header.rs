//! Metadata and configuration for las files.

use {Bounds, GpsTimeType, Result, Transform, Vector, Version, Vlr, raw};
use chrono::{Date, Utc};
use point::Format;
use std::collections::HashMap;
use utils::{AsLasStr, FromLasStr};

quick_error! {
    /// Header-specific errors.
    #[derive(Clone, Copy, Debug)]
    pub enum Error {
        /// The header size, as computed, is too large.
        TooLarge(len: usize) {
            description("the header is too large to convert to a raw header")
            display("the header is too large to convert to a raw header: {} bytes", len)
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
        /// The offset to point data is too large.
        OffsetToPointDataTooLarge(offset: usize) {
            description("the offset to the point data is too large")
            display("the offset to the point data is too large: {}", offset)
        }
    }
}

/// Metadata describing the layout, source, and interpretation of the points.
#[derive(Clone, Debug)]
pub struct Header {
    /// A project-wide unique ID for the file.
    pub file_source_id: u16,

    /// The time type for GPS time.
    pub gps_time_type: GpsTimeType,

    /// Optional globally-unique identifier.
    pub guid: [u8; 16],

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
    pub number_of_points: u32,

    /// The number of points of each return type (1-5).
    pub number_of_points_by_return: HashMap<u8, u32>,

    /// Variable length records.
    pub vlrs: Vec<Vlr>,
}

impl Header {
    /// Creates a new header from a raw header, vlrs, and vlr padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{raw, Header};
    /// let raw_header = raw::Header::default();
    /// let header = Header::new(raw_header, vec![], vec![]).unwrap();
    /// ```
    pub fn new(header: raw::Header, vlrs: Vec<Vlr>, vlr_padding: Vec<u8>) -> Result<Header> {
        use chrono::TimeZone;

        Ok(Header {
            file_source_id: header.file_source_id,
            gps_time_type: header.global_encoding.into(),
            date: Utc.yo_opt(
                header.file_creation_year as i32,
                header.file_creation_day_of_year as u32,
            ).single(),
            generating_software: header
                .generating_software
                .as_ref()
                .as_las_str()?
                .to_string(),
            guid: header.guid,
            padding: header.padding,
            vlr_padding: vlr_padding,
            point_format: Format::new(header.point_data_format_id)?,
            number_of_points: header.number_of_point_records,
            number_of_points_by_return: header
                .number_of_points_by_return
                .into_iter()
                .enumerate()
                .map(|(i, &n)| (i as u8 + 1, n))
                .collect(),
            system_identifier: header.system_identifier.as_ref().as_las_str()?.to_string(),
            transforms: Vector {
                x: Transform {
                    scale: header.x_scale_factor,
                    offset: header.x_offset,
                },
                y: Transform {
                    scale: header.y_scale_factor,
                    offset: header.y_offset,
                },
                z: Transform {
                    scale: header.z_scale_factor,
                    offset: header.z_offset,
                },
            },
            bounds: Bounds {
                min: Vector {
                    x: header.min_x,
                    y: header.min_y,
                    z: header.min_z,
                },
                max: Vector {
                    x: header.max_x,
                    y: header.max_y,
                    z: header.max_z,
                },
            },
            version: header.version,
            vlrs: vlrs,
        })
    }

    /// Converts this header into a raw header.
    ///
    /// This method does some rules-checking.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let raw_header = Header::default().to_raw().unwrap();
    /// ```
    pub fn to_raw(&self) -> Result<raw::Header> {
        use chrono::Datelike;

        Ok(raw::Header {
            file_signature: raw::LASF,
            file_source_id: self.file_source_id,
            global_encoding: self.gps_time_type.into(),
            guid: self.guid,
            version: self.version,
            system_identifier: self.raw_system_identifier()?,
            generating_software: self.raw_generating_software()?,
            file_creation_day_of_year: self.date.map_or(0, |d| d.ordinal() as u16),
            file_creation_year: self.date.map_or(0, |d| d.year() as u16),
            header_size: self.header_size()?,
            offset_to_point_data: self.offset_to_point_data()?,
            number_of_variable_length_records: self.number_of_variable_length_records()?,
            point_data_format_id: self.point_format.to_u8()?,
            // TODO extra bytes
            point_data_record_length: self.point_format.len(),
            number_of_point_records: self.number_of_points,
            number_of_points_by_return: self.raw_number_of_points_by_return()?,
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
            start_of_first_evlr: None,
            number_of_evlrs: None,
            // TODO we could populate these
            number_of_point_records_64bit: None,
            number_of_points_by_return_64bit: None,
            padding: self.padding.clone(),
        })
    }

    fn number_of_variable_length_records(&self) -> Result<u32> {
        use std::u32;

        if self.vlrs.len() > u32::MAX as usize {
            Err(Error::TooManyVlrs(self.vlrs.len()).into())
        } else {
            Ok(self.vlrs.len() as u32)
        }
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

        let vlr_len = self.vlrs.iter().fold(0, |acc, vlr| acc + vlr.len());
        let offset = self.header_size()? as usize + vlr_len + self.vlr_padding.len();
        if offset > u32::MAX as usize {
            Err(Error::OffsetToPointDataTooLarge(offset).into())
        } else {
            Ok(offset as u32)
        }
    }

    fn raw_system_identifier(&self) -> Result<[u8; 32]> {
        let mut system_identifier = [0; 32];
        system_identifier.as_mut().from_las_str(
            &self.system_identifier,
        )?;
        Ok(system_identifier)
    }

    fn raw_generating_software(&self) -> Result<[u8; 32]> {
        let mut generating_software = [0; 32];
        generating_software.as_mut().from_las_str(
            &self.generating_software,
        )?;
        Ok(generating_software)
    }

    fn raw_number_of_points_by_return(&self) -> Result<[u32; 5]> {
        let mut number_of_points_by_return = [0; 5];
        for (&i, &n) in &self.number_of_points_by_return {
            if i > 5 {
                return Err(::Error::InvalidReturnNumber(i, Some(self.version)));
            } else if i > 0 {
                number_of_points_by_return[i as usize - 1] = n;
            }
        }
        Ok(number_of_points_by_return)
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            file_source_id: 0,
            gps_time_type: GpsTimeType::Week,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_day_no_date() {
        let raw_header = raw::Header {
            file_creation_day_of_year: 0,
            ..Default::default()
        };
        let header = Header::new(raw_header, Vec::new(), Vec::new()).unwrap();
        assert!(header.date.is_none());
    }

    #[test]
    fn no_year_no_date() {
        let raw_header = raw::Header {
            file_creation_year: 0,
            ..Default::default()
        };
        let header = Header::new(raw_header, Vec::new(), Vec::new()).unwrap();
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
}
