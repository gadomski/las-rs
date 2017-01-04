use {Bounds, Error, Header, Result, Transform, Vector, Vlr};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{Datelike, TimeZone, UTC};
use header::{GpsTimeType, HEADER_SIZE};
use std::io::{Read, Write};
use utils::{FromLasStr, ToLasStr};

/// A raw header that maps directly onto the file structure.
#[derive(Debug)]
#[allow(missing_docs)]
pub struct RawHeader {
    pub file_signature: [u8; 4],
    pub file_source_id: u16,
    pub global_encoding: u16,
    pub guid: [u8; 16],
    pub version_major: u8,
    pub version_minor: u8,
    pub system_identifier: [u8; 32],
    pub generating_software: [u8; 32],
    pub file_creation_day_of_year: u16,
    pub file_creation_year: u16,
    pub header_size: u16,
    pub offset_to_point_data: u32,
    pub number_of_variable_length_records: u32,
    pub point_data_format_id: u8,
    pub point_data_record_length: u16,
    pub number_of_point_records: u32,
    pub number_of_points_by_return: [u32; 5],
    pub x_scale_factor: f64,
    pub y_scale_factor: f64,
    pub z_scale_factor: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub z_offset: f64,
    pub max_x: f64,
    pub min_x: f64,
    pub max_y: f64,
    pub min_y: f64,
    pub max_z: f64,
    pub min_z: f64,
    pub padding: Vec<u8>,
}

impl RawHeader {
    /// Converts this raw header into a `Header`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::header::RawHeader;
    /// let raw_header = RawHeader { ..Default::default() };
    /// let header = raw_header.into_header(Vec::new(), Vec::new()).unwrap();
    /// ```
    pub fn into_header(self, vlrs: Vec<Vlr>, vlr_padding: Vec<u8>) -> Result<Header> {
        let gps_time_type = if (self.global_encoding & 1) == 1 {
            GpsTimeType::Week
        } else {
            GpsTimeType::Standard
        };
        if self.header_size < HEADER_SIZE {
            return Err(Error::HeaderSizeTooSmall(self.header_size));
        }
        let vlr_len = vlrs.iter().fold(0, |acc, vlr| acc + vlr.len());
        if self.offset_to_point_data < self.header_size as u32 + vlr_len {
            return Err(Error::OffsetToDataTooSmall(self.offset_to_point_data));
        }
        Ok(Header {
            file_source_id: self.file_source_id,
            gps_time_type: gps_time_type,
            date: UTC.yo_opt(self.file_creation_year as i32,
                        self.file_creation_day_of_year as u32)
                .single(),
            generating_software: self.generating_software.as_ref().to_las_str()?.to_string(),
            guid: self.guid,
            // TODO las 1.4 header
            padding: self.padding,
            vlr_padding: vlr_padding,
            point_format: self.point_data_format_id.into(),
            number_of_points: self.number_of_point_records,
            number_of_points_by_return: self.number_of_points_by_return,
            system_identifier: self.system_identifier.as_ref().to_las_str()?.to_string(),
            transforms: Vector {
                x: Transform {
                    scale: self.x_scale_factor,
                    offset: self.x_offset,
                },
                y: Transform {
                    scale: self.y_scale_factor,
                    offset: self.y_offset,
                },
                z: Transform {
                    scale: self.z_scale_factor,
                    offset: self.z_offset,
                },
            },
            bounds: Bounds {
                min: Vector {
                    x: self.min_x,
                    y: self.min_y,
                    z: self.min_z,
                },
                max: Vector {
                    x: self.max_x,
                    y: self.max_y,
                    z: self.max_z,
                },
            },
            version: (self.version_major, self.version_minor),
            vlrs: vlrs,
        })
    }

    /// Returns true if this raw header is for compressed las data.
    ///
    /// Though this isn't part of the las spec, the two high bits of the point data format id have
    /// been used to indicate compressed data, particularily by the laszip library.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::header::RawHeader;
    /// let mut raw_header = RawHeader { ..Default::default() };
    /// assert!(!raw_header.is_compressed());
    /// raw_header.point_data_format_id = 131;
    /// assert!(raw_header.is_compressed());
    /// ```
    pub fn is_compressed(&self) -> bool {
        (self.point_data_format_id >> 7) == 1
    }
}

impl Default for RawHeader {
    fn default() -> RawHeader {
        RawHeader {
            file_signature: *b"LASF",
            file_source_id: 0,
            global_encoding: 0,
            guid: [0; 16],
            version_major: 0,
            version_minor: 0,
            system_identifier: [0; 32],
            generating_software: [0; 32],
            file_creation_day_of_year: 0,
            file_creation_year: 0,
            header_size: HEADER_SIZE,
            offset_to_point_data: HEADER_SIZE as u32,
            number_of_variable_length_records: 0,
            point_data_format_id: 0,
            point_data_record_length: 0,
            number_of_point_records: 0,
            number_of_points_by_return: [0; 5],
            x_scale_factor: 0.,
            y_scale_factor: 0.,
            z_scale_factor: 0.,
            x_offset: 0.,
            y_offset: 0.,
            z_offset: 0.,
            max_x: 0.,
            min_x: 0.,
            max_y: 0.,
            min_y: 0.,
            max_z: 0.,
            min_z: 0.,
            padding: Vec::new(),
        }
    }
}

impl Header {
    /// Converts this header into a `RawHeader.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Header;
    /// let raw_header = Header { ..Default::default() }.to_raw_header().unwrap();
    /// ```
    pub fn to_raw_header(&self) -> Result<RawHeader> {
        let global_encoding = match self.gps_time_type {
            GpsTimeType::Week => 0,
            GpsTimeType::Standard => 1,
        };
        let mut system_identifier = [0; 32];
        system_identifier.as_mut().from_las_str(&self.system_identifier)?;
        let mut generating_software = [0; 32];
        generating_software.as_mut().from_las_str(&self.generating_software)?;
        let vlr_len = self.vlrs.iter().fold(0, |acc, vlr| acc + vlr.len());
        Ok(RawHeader {
            file_signature: *b"LASF",
            file_source_id: self.file_source_id,
            global_encoding: global_encoding,
            guid: self.guid,
            version_major: self.version.0,
            version_minor: self.version.1,
            system_identifier: system_identifier,
            generating_software: generating_software,
            file_creation_day_of_year: self.date.map_or(0, |d| d.ordinal() as u16),
            file_creation_year: self.date.map_or(0, |d| d.year() as u16),
            header_size: HEADER_SIZE + self.padding.len() as u16,
            offset_to_point_data: HEADER_SIZE as u32 + self.padding.len() as u32 + vlr_len +
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
            padding: self.padding.clone(),
        })
    }
}

/// Reads a raw header.
pub trait ReadRawHeader {
    /// Reads a raw header.
    ///
    /// # Examples
    ///
    /// `Read` implements `ReadRawHeader`.
    ///
    /// ```
    /// use std::fs::File;
    /// use las::header::ReadRawHeader;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// let header = file.read_raw_header().unwrap();
    /// ```
    fn read_raw_header(&mut self) -> Result<RawHeader>;
}

impl<R: Read> ReadRawHeader for R {
    fn read_raw_header(&mut self) -> Result<RawHeader> {
        let mut file_signature = [0; 4];
        self.read_exact(&mut file_signature)?;
        let file_source_id = self.read_u16::<LittleEndian>()?;
        let global_encoding = self.read_u16::<LittleEndian>()?;
        let mut guid = [0; 16];
        self.read_exact(&mut guid)?;
        let version_major = self.read_u8()?;
        let version_minor = self.read_u8()?;
        let mut system_identifier = [0; 32];
        self.read_exact(&mut system_identifier)?;
        let mut generating_software = [0; 32];
        self.read_exact(&mut generating_software)?;
        let file_creation_day_of_year = self.read_u16::<LittleEndian>()?;
        let file_creation_year = self.read_u16::<LittleEndian>()?;
        let header_size = self.read_u16::<LittleEndian>()?;
        let offset_to_point_data = self.read_u32::<LittleEndian>()?;
        let number_of_variable_length_records = self.read_u32::<LittleEndian>()?;
        let point_data_format_id = self.read_u8()?;
        let point_data_record_length = self.read_u16::<LittleEndian>()?;
        let number_of_point_records = self.read_u32::<LittleEndian>()?;
        let mut number_of_points_by_return = [0; 5];
        for n in number_of_points_by_return.iter_mut() {
            *n = self.read_u32::<LittleEndian>()?;
        }
        let x_scale_factor = self.read_f64::<LittleEndian>()?;
        let y_scale_factor = self.read_f64::<LittleEndian>()?;
        let z_scale_factor = self.read_f64::<LittleEndian>()?;
        let x_offset = self.read_f64::<LittleEndian>()?;
        let y_offset = self.read_f64::<LittleEndian>()?;
        let z_offset = self.read_f64::<LittleEndian>()?;
        let max_x = self.read_f64::<LittleEndian>()?;
        let min_x = self.read_f64::<LittleEndian>()?;
        let max_y = self.read_f64::<LittleEndian>()?;
        let min_y = self.read_f64::<LittleEndian>()?;
        let max_z = self.read_f64::<LittleEndian>()?;
        let min_z = self.read_f64::<LittleEndian>()?;
        // TODO las 1.4
        let padding = if header_size > HEADER_SIZE {
            let mut bytes = vec![0; (header_size - HEADER_SIZE) as usize];
            self.read_exact(&mut bytes)?;
            bytes
        } else {
            Vec::new()
        };
        Ok(RawHeader {
            file_signature: file_signature,
            file_source_id: file_source_id,
            global_encoding: global_encoding,
            guid: guid,
            version_major: version_major,
            version_minor: version_minor,
            system_identifier: system_identifier,
            generating_software: generating_software,
            file_creation_day_of_year: file_creation_day_of_year,
            file_creation_year: file_creation_year,
            header_size: header_size,
            offset_to_point_data: offset_to_point_data,
            number_of_variable_length_records: number_of_variable_length_records,
            point_data_format_id: point_data_format_id,
            point_data_record_length: point_data_record_length,
            number_of_point_records: number_of_point_records,
            number_of_points_by_return: number_of_points_by_return,
            x_scale_factor: x_scale_factor,
            y_scale_factor: y_scale_factor,
            z_scale_factor: z_scale_factor,
            x_offset: x_offset,
            y_offset: y_offset,
            z_offset: z_offset,
            max_x: max_x,
            min_x: min_x,
            max_y: max_y,
            min_y: min_y,
            max_z: max_z,
            min_z: min_z,
            padding: padding,
        })
    }
}

/// Writes a raw header.
pub trait WriteRawHeader {
    /// Writes a raw header.
    ///
    /// # Examples
    ///
    /// `Write` implements `WriteRawHeader`.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::header::WriteRawHeader;
    /// let mut cursor = Cursor::new(Vec::new());
    /// cursor.write_raw_header(&Default::default()).unwrap();
    /// ```
    fn write_raw_header(&mut self, raw_header: &RawHeader) -> Result<()>;
}

impl<W: Write> WriteRawHeader for W {
    fn write_raw_header(&mut self, raw_header: &RawHeader) -> Result<()> {
        self.write_all(&raw_header.file_signature)?;
        self.write_u16::<LittleEndian>(raw_header.file_source_id)?;
        self.write_u16::<LittleEndian>(raw_header.global_encoding)?;
        self.write_all(&raw_header.guid)?;
        self.write_u8(raw_header.version_major)?;
        self.write_u8(raw_header.version_minor)?;
        self.write_all(&raw_header.system_identifier)?;
        self.write_all(&raw_header.generating_software)?;
        self.write_u16::<LittleEndian>(raw_header.file_creation_day_of_year)?;
        self.write_u16::<LittleEndian>(raw_header.file_creation_year)?;
        self.write_u16::<LittleEndian>(raw_header.header_size)?;
        self.write_u32::<LittleEndian>(raw_header.offset_to_point_data)?;
        self.write_u32::<LittleEndian>(raw_header.number_of_variable_length_records)?;
        self.write_u8(raw_header.point_data_format_id)?;
        self.write_u16::<LittleEndian>(raw_header.point_data_record_length)?;
        self.write_u32::<LittleEndian>(raw_header.number_of_point_records)?;
        for n in raw_header.number_of_points_by_return.iter() {
            self.write_u32::<LittleEndian>(*n)?;
        }
        self.write_f64::<LittleEndian>(raw_header.x_scale_factor)?;
        self.write_f64::<LittleEndian>(raw_header.y_scale_factor)?;
        self.write_f64::<LittleEndian>(raw_header.z_scale_factor)?;
        self.write_f64::<LittleEndian>(raw_header.x_offset)?;
        self.write_f64::<LittleEndian>(raw_header.y_offset)?;
        self.write_f64::<LittleEndian>(raw_header.z_offset)?;
        self.write_f64::<LittleEndian>(raw_header.max_x)?;
        self.write_f64::<LittleEndian>(raw_header.min_x)?;
        self.write_f64::<LittleEndian>(raw_header.max_y)?;
        self.write_f64::<LittleEndian>(raw_header.min_y)?;
        self.write_f64::<LittleEndian>(raw_header.max_z)?;
        self.write_f64::<LittleEndian>(raw_header.min_z)?;
        if !raw_header.padding.is_empty() {
            self.write_all(&raw_header.padding)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_day_no_date() {
        let raw_header = RawHeader { file_creation_day_of_year: 0, ..Default::default() };
        let header = raw_header.into_header(Vec::new(), Vec::new()).unwrap();
        assert!(header.date.is_none());
    }

    #[test]
    fn no_year_no_date() {
        let raw_header = RawHeader { file_creation_year: 0, ..Default::default() };
        let header = raw_header.into_header(Vec::new(), Vec::new()).unwrap();
        assert!(header.date.is_none());
    }

    #[test]
    fn raw_header_is_compressed() {
        let mut raw_header = RawHeader { point_data_format_id: 0, ..Default::default() };
        assert!(!raw_header.is_compressed());
        raw_header.point_data_format_id = 131;
        assert!(raw_header.is_compressed());
    }
}
