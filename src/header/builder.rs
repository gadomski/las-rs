use {Bounds, GpsTimeType, Header, Result, Transform, Vector, Version, Vlr, raw};
use chrono::{Date, Utc};
use header::Error;
use point::Format;
use std::collections::HashMap;
use uuid::Uuid;

/// Builds headers.
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// The date of file creation.
    pub date: Option<Date<Utc>>,

    /// The file source id, sometimes the flight line.
    pub file_source_id: u16,

    /// The software that created this file.
    pub generating_software: String,

    /// The type of gps time, either week or standard.
    pub gps_time_type: GpsTimeType,

    /// A globally unique identifier.
    pub guid: Uuid,

    /// Are the return numbers in this file syntheetic?
    pub has_synthetic_return_numbers: bool,

    /// Bytes after the header but before the vlrs.
    pub padding: Vec<u8>,

    /// The format that the points will be written in.
    pub point_format: Format,

    /// The bytes after the points but before any evlrs.
    ///
    /// Discouraged.
    pub point_padding: Vec<u8>,

    /// The system that generated the points.
    pub system_identifier: String,

    /// The scale and offset that will be used to convert coordinates to `i16`s to write in the
    /// file.
    pub transforms: Vector<Transform>,

    /// The las version.
    pub version: Version,

    /// The bytes after the vlrs but before the points.
    pub vlr_padding: Vec<u8>,

    /// The variable length records.
    pub vlrs: Vec<Vlr>,

    /// The extended variable length records.
    pub evlrs: Vec<Vlr>,

    number_of_points_by_return: HashMap<u8, u64>,
    number_of_points: u64,
    bounds: Bounds,
}

impl Builder {
    /// Creates a new builder from a raw header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Builder;
    /// let builder = Builder::new(Default::default()).unwrap();
    /// ```
    pub fn new(raw_header: raw::Header) -> Result<Builder> {
        use chrono::TimeZone;
        use utils::AsLasStr;

        let number_of_points = if raw_header.number_of_point_records > 0 {
            u64::from(raw_header.number_of_point_records)
        } else {
            raw_header
                .large_file
                .map(|l| l.number_of_point_records)
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
        let mut point_format = Format::new(raw_header.point_data_record_format)?;
        let n = point_format.len();
        if raw_header.point_data_record_length < n {
            return Err(
                Error::PointDataRecordLength(point_format, raw_header.point_data_record_length)
                    .into(),
            );
        } else if n < raw_header.point_data_record_length {
            point_format.extra_bytes = raw_header.point_data_record_length - n;
        }
        Ok(Builder {
            date: Utc.yo_opt(
                i32::from(raw_header.file_creation_year),
                u32::from(raw_header.file_creation_day_of_year),
            ).single(),
            point_padding: Vec::new(),
            evlrs: Vec::new(),
            file_source_id: raw_header.file_source_id,
            generating_software: raw_header
                .generating_software
                .as_ref()
                .as_las_str()?
                .to_string(),
            gps_time_type: raw_header.global_encoding.into(),
            guid: Uuid::from_bytes(&raw_header.guid).unwrap(),
            has_synthetic_return_numbers: raw_header.global_encoding & 8 == 8,
            padding: raw_header.padding,
            point_format: point_format,
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
            version: raw_header.version,
            vlr_padding: Vec::new(),
            vlrs: Vec::new(),
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
            number_of_points: number_of_points,
            number_of_points_by_return: number_of_points_by_return,
        })
    }

    /// Converts this builder into a `Header`.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Builder;
    /// let header = Builder::new(Default::default()).unwrap().into_header().unwrap();
    /// ```
    pub fn into_header(mut self) -> Result<Header> {
        use raw::POINT_DATA_START_SIGNATURE;
        use feature::{Evlrs, FileSourceId, GpsStandardTime, SyntheticReturnNumbers};

        let n = self.vlr_padding.len();
        if self.version.requires_point_data_start_signature() &&
            (n < 2 || !(self.vlr_padding[n - 2..] == POINT_DATA_START_SIGNATURE))
        {
            self.vlr_padding.extend(&POINT_DATA_START_SIGNATURE);
        }
        if self.file_source_id != 0 {
            self.version.verify_support_for::<FileSourceId>()?;
        }
        if self.has_synthetic_return_numbers {
            self.version.verify_support_for::<SyntheticReturnNumbers>()?;
        }
        if self.gps_time_type.is_standard() {
            self.version.verify_support_for::<GpsStandardTime>()?;
        }
        // TODO check waveforms
        if !self.version.supports_point_format(self.point_format) {
            return Err(Error::Format(self.version, self.point_format).into());
        }
        let mut vlrs = Vec::new();
        let mut evlrs = Vec::new();
        for evlr in self.evlrs {
            if self.version.supports::<Evlrs>() || evlr.has_large_data() {
                evlrs.push(evlr);
            } else {
                vlrs.push(evlr);
            }
        }
        for vlr in self.vlrs {
            if vlr.has_large_data() {
                evlrs.push(vlr);
            } else {
                vlrs.push(vlr);
            }
        }
        if !evlrs.is_empty() {
            self.version.verify_support_for::<Evlrs>()?;
        } else if !self.point_padding.is_empty() {
            return Err(Error::PointPadding.into());
        }
        let header = Header {
            bounds: self.bounds,
            date: self.date,
            evlrs: evlrs,
            file_source_id: self.file_source_id,
            generating_software: self.generating_software,
            gps_time_type: self.gps_time_type,
            guid: self.guid,
            has_synthetic_return_numbers: self.has_synthetic_return_numbers,
            number_of_points: self.number_of_points,
            number_of_points_by_return: self.number_of_points_by_return,
            padding: self.padding,
            point_format: self.point_format,
            point_padding: self.point_padding,
            system_identifier: self.system_identifier,
            transforms: self.transforms,
            version: self.version,
            vlr_padding: self.vlr_padding,
            vlrs: vlrs,
        };
        Ok(header)
    }
}

impl<V: Into<Version>> From<V> for Builder {
    fn from(version: V) -> Builder {
        let mut builder = Builder::default();
        builder.version = version.into();
        builder
    }
}

impl From<Header> for Builder {
    fn from(header: Header) -> Builder {
        Builder {
            bounds: header.bounds,
            date: header.date,
            evlrs: header.evlrs,
            file_source_id: header.file_source_id,
            generating_software: header.generating_software,
            gps_time_type: header.gps_time_type,
            guid: header.guid,
            has_synthetic_return_numbers: header.has_synthetic_return_numbers,
            number_of_points: header.number_of_points,
            number_of_points_by_return: header.number_of_points_by_return,
            padding: header.padding,
            point_format: header.point_format,
            point_padding: header.point_padding,
            system_identifier: header.system_identifier,
            transforms: header.transforms,
            version: header.version,
            vlr_padding: header.vlr_padding,
            vlrs: header.vlrs,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_day_no_date() {
        let raw_header = raw::Header {
            file_creation_day_of_year: 0,
            ..Default::default()
        };
        let builder = Builder::new(raw_header).unwrap();
        assert!(builder.date.is_none());
    }

    #[test]
    fn no_year_no_date() {
        let raw_header = raw::Header {
            file_creation_year: 0,
            ..Default::default()
        };
        let builder = Builder::new(raw_header).unwrap();
        assert!(builder.date.is_none());
    }

    // TODO assert wkt properties

    #[test]
    fn evlr_downgrade() {
        let mut builder = Builder::from((1, 2));
        builder.evlrs.push(Vlr::default());
        let header = builder.into_header().unwrap();
        assert_eq!(1, header.vlrs().len());
        assert_eq!(0, header.evlrs().len());
    }

    #[test]
    fn evlr_upgrade() {
        let mut builder = Builder::from((1, 4));
        let mut vlr = Vlr::default();
        vlr.data = vec![0; ::std::u16::MAX as usize + 1];
        builder.vlrs.push(vlr);
        let header = builder.into_header().unwrap();
        assert_eq!(0, header.vlrs().len());
        assert_eq!(1, header.evlrs().len());
    }

    #[test]
    fn point_padding_no_evlrs() {
        let mut builder = Builder::from((1, 4));
        builder.point_padding = vec![0];
        assert!(builder.into_header().is_err());
    }

    #[test]
    fn point_data_start_signature() {
        let mut builder = Builder::from((1, 0));
        builder.vlr_padding = vec![42];
        let header = builder.into_header().unwrap();
        assert_eq!(vec![42, 0xCC, 0xDD], *header.vlr_padding());

        let builder = Builder::from((1, 2));
        assert!(builder.into_header().unwrap().vlr_padding().is_empty());
    }
}
