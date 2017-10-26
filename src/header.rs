use {Bounds, GpsTimeType, Result, Transform, Vector, Version, Vlr, raw};
use chrono::{Date, Utc};
use point::Format;

/// Metadata describing the layout, source, and interpretation of the points.
///
/// This structure is a higher-level representation of the header data, but can be converted into a
/// `RawHeader` to write actual LAS headers.
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
    ///
    /// TODO extra bytes.
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
    pub number_of_points_by_return: [u32; 5],
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
        use Error;
        use chrono::TimeZone;
        use utils::ToLasStr;

        let gps_time_type = if (header.global_encoding & 1) == 1 {
            GpsTimeType::Week
        } else {
            GpsTimeType::Standard
        };
        // TODO check header size
        let vlr_len = vlrs.iter().fold(0, |acc, vlr| acc + vlr.len());
        if header.offset_to_point_data < header.header_size as u32 + vlr_len {
            return Err(Error::OffsetToDataTooSmall(header.offset_to_point_data));
        }
        Ok(Header {
            file_source_id: header.file_source_id,
            gps_time_type: gps_time_type,
            date: Utc.yo_opt(
                header.file_creation_year as i32,
                header.file_creation_day_of_year as u32,
            ).single(),
            generating_software: header
                .generating_software
                .as_ref()
                .to_las_str()?
                .to_string(),
            guid: header.guid,
            // TODO las 1.4 header
            padding: header.padding,
            vlr_padding: vlr_padding,
            point_format: header.point_data_format_id.into(),
            number_of_points: header.number_of_point_records,
            number_of_points_by_return: header.number_of_points_by_return,
            system_identifier: header.system_identifier.as_ref().to_las_str()?.to_string(),
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
            version: header.version.into(),
            vlrs: vlrs,
        })
    }

    /// Returns the length of this header.
    ///
    /// This includes both the length of the written header and the padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let header = Header::default();
    /// assert_eq!(227, header.len());
    /// let header = Header { padding: vec![0], ..Default::default() };
    /// assert_eq!(228, header.len());
    /// ```
    pub fn len(&self) -> u16 {
        self.version.header_size() + self.padding.len() as u16
    }

    /// Converts this header into a `RawHeader.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    /// let raw_header = Header::default().to_raw().unwrap();
    /// ```
    pub fn to_raw(&self) -> Result<raw::Header> {
        use utils::FromLasStr;
        use chrono::Datelike;

        let global_encoding = match self.gps_time_type {
            GpsTimeType::Week => 0,
            GpsTimeType::Standard => 1,
        };
        let mut system_identifier = [0; 32];
        system_identifier.as_mut().from_las_str(
            &self.system_identifier,
        )?;
        let mut generating_software = [0; 32];
        generating_software.as_mut().from_las_str(
            &self.generating_software,
        )?;
        let vlr_len = self.vlrs.iter().fold(0, |acc, vlr| acc + vlr.len());
        Ok(raw::Header {
            file_signature: raw::LASF,
            file_source_id: self.file_source_id,
            global_encoding: global_encoding,
            guid: self.guid,
            version: self.version,
            system_identifier: system_identifier,
            generating_software: generating_software,
            file_creation_day_of_year: self.date.map_or(0, |d| d.ordinal() as u16),
            file_creation_year: self.date.map_or(0, |d| d.year() as u16),
            header_size: self.version.header_size() + self.padding.len() as u16,
            offset_to_point_data: self.version.header_size() as u32 + self.padding.len() as u32 +
                vlr_len +
                self.vlr_padding.len() as u32,
            number_of_variable_length_records: self.vlrs.len() as u32,
            point_data_format_id: self.point_format.into(),
            // TODO extra bytes
            point_data_record_length: self.point_format.len(),
            number_of_point_records: self.number_of_points,
            number_of_points_by_return: self.number_of_points_by_return,
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
            number_of_points_by_return: [0; 5],
            padding: Vec::new(),
            vlr_padding: Vec::new(),
            point_format: 0.into(),
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
}
