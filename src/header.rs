//! `las` headers describe the point format, data bounds, and other metadata.

use std::io::{Read, Write};
use std::fmt;
use std::iter::repeat;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use Result;
use error::Error;
use io::read_full;

/// This constant value is the bytes in your normal header. There are worlds where people put crap
/// into headers that don't belong there, so we have to guard against this.
const DEFAULT_BYTES_IN_HEADER: u16 = 227;

/// A las header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Header {
    /// The las file signature.
    ///
    /// Should always be "LASF".
    pub file_signature: [u8; 4],
    /// A numeric identifier for this file.
    pub file_source_id: u16,
    /// Unused in early version, and exapanded to help with GPS time offsets in later versions.
    pub global_encoding: u16,
    /// The first of four parts of the project id.
    pub guid_data_1: u32,
    /// The second of four parts of the project id.
    pub guid_data_2: u16,
    /// The third of four parts of the project id.
    pub guid_data_3: u16,
    /// The fourth of four parts of the project id.
    pub guid_data_4: [u8; 8],
    /// The las major version.
    ///
    /// Should always be 1.
    pub version_major: u8,
    /// The las minor version.
    pub version_minor: u8,
    /// Generally the hardware system that created the las file.
    ///
    /// Can also be the algorithm that produced the lasfile. This field is poorly defined.
    pub system_identifier: [u8; 32],
    /// The software the generated the las file.
    pub generating_software: [u8; 32],
    /// The day of the year, indexed to 1.
    pub file_creation_day_of_year: u16,
    /// The year of file creation.
    pub file_creation_year: u16,
    /// The size of the las header.
    ///
    /// Softwares are technically allowed to add custom extensions to the las header, which would
    /// then affect this header size, but that is discouraged as such support is not universal.
    pub header_size: u16,
    /// The byte offset to the beginning of point data.
    ///
    /// Includes the size of the header and the variable length records.
    pub offset_to_point_data: u32,
    /// The number of variable length records.
    pub number_of_variable_length_records: u32,
    /// The point data format.
    pub point_data_format: PointDataFormat,
    /// The length of one point data record, in bytes.
    pub point_data_record_length: u16,
    /// The total number of point records.
    pub number_of_point_records: u32,
    /// The number of point records of each return number.
    ///
    /// This only supports five returns per pulse.
    pub number_of_points_by_return: [u32; 5],
    /// The x scale factor for each point.
    pub x_scale_factor: f64,
    /// The y scale factor for each point.
    pub y_scale_factor: f64,
    /// The z scale factor for each point.
    pub z_scale_factor: f64,
    /// The x offset for each point.
    pub x_offset: f64,
    /// The y offset for each point.
    pub y_offset: f64,
    /// The z offset for each point.
    pub z_offset: f64,
    /// The maximum x value.
    pub x_max: f64,
    /// The minimum x value.
    pub x_min: f64,
    /// The maximum y value.
    pub y_max: f64,
    /// The minimum y value.
    pub y_min: f64,
    /// The maximum z value.
    pub z_max: f64,
    /// The minimum z value.
    pub z_min: f64,
}

impl Header {
    /// Reads a header from a `Read`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use las::header::Header;
    /// let ref mut file = File::open("data/1.0_0.las").unwrap();
    /// let header = Header::read_from(file);
    /// ```
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Header> {
        let mut header = Header::new();
        try!(read_full(reader, &mut header.file_signature));
        header.file_source_id = try!(reader.read_u16::<LittleEndian>());
        header.global_encoding = try!(reader.read_u16::<LittleEndian>());
        header.guid_data_1 = try!(reader.read_u32::<LittleEndian>());
        header.guid_data_2 = try!(reader.read_u16::<LittleEndian>());
        header.guid_data_3 = try!(reader.read_u16::<LittleEndian>());
        try!(read_full(reader, &mut header.guid_data_4));
        header.version_major = try!(reader.read_u8());
        header.version_minor = try!(reader.read_u8());
        try!(read_full(reader, &mut header.system_identifier));
        try!(read_full(reader, &mut header.generating_software));
        header.file_creation_day_of_year = try!(reader.read_u16::<LittleEndian>());
        header.file_creation_year = try!(reader.read_u16::<LittleEndian>());
        header.header_size = try!(reader.read_u16::<LittleEndian>());
        header.offset_to_point_data = try!(reader.read_u32::<LittleEndian>());
        header.number_of_variable_length_records = try!(reader.read_u32::<LittleEndian>());
        header.point_data_format = try!(PointDataFormat::from_u8(try!(reader.read_u8())));
        header.point_data_record_length = try!(reader.read_u16::<LittleEndian>());
        header.number_of_point_records = try!(reader.read_u32::<LittleEndian>());
        for n in &mut header.number_of_points_by_return {
            *n = try!(reader.read_u32::<LittleEndian>());
        }
        header.x_scale_factor = try!(reader.read_f64::<LittleEndian>());
        header.y_scale_factor = try!(reader.read_f64::<LittleEndian>());
        header.z_scale_factor = try!(reader.read_f64::<LittleEndian>());
        header.x_offset = try!(reader.read_f64::<LittleEndian>());
        header.y_offset = try!(reader.read_f64::<LittleEndian>());
        header.z_offset = try!(reader.read_f64::<LittleEndian>());
        header.x_max = try!(reader.read_f64::<LittleEndian>());
        header.x_min = try!(reader.read_f64::<LittleEndian>());
        header.y_max = try!(reader.read_f64::<LittleEndian>());
        header.y_min = try!(reader.read_f64::<LittleEndian>());
        header.z_max = try!(reader.read_f64::<LittleEndian>());
        header.z_min = try!(reader.read_f64::<LittleEndian>());
        Ok(header)
    }

    /// Creates a new, empty header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::header::Header;
    /// let header = Header::new();
    /// ```
    pub fn new() -> Header {
        let name_and_version = format!("las-rs {}", env!("CARGO_PKG_VERSION")).into_bytes();
        let mut generating_software = [0; 32];
        for (c, b) in name_and_version.iter()
                                      .chain(repeat(&0).take(generating_software.len() -
                                                             name_and_version.len()))
                                      .zip(generating_software.iter_mut()) {
            *b = *c;
        }
        Header {
            file_signature: *b"LASF",
            file_source_id: 0,
            global_encoding: 0,
            guid_data_1: 0,
            guid_data_2: 0,
            guid_data_3: 0,
            guid_data_4: [0; 8],
            version_major: 1,
            version_minor: 0,
            system_identifier: [0; 32],
            generating_software: generating_software,
            file_creation_day_of_year: 0,
            file_creation_year: 0,
            header_size: 0,
            offset_to_point_data: 0,
            number_of_variable_length_records: 0,
            point_data_format: PointDataFormat(0),
            point_data_record_length: 0,
            number_of_point_records: 0,
            number_of_points_by_return: [0; 5],
            x_scale_factor: 1.0,
            y_scale_factor: 1.0,
            z_scale_factor: 1.0,
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            x_max: 0.0,
            x_min: 0.0,
            y_max: 0.0,
            y_min: 0.0,
            z_max: 0.0,
            z_min: 0.0,
        }
    }

    /// Writes this header to a `Write`.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::header::Header;
    /// let header = Header::new();
    /// let ref mut writer = Cursor::new(Vec::new());
    /// let bytes_written = header.write_to(writer).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<u16> {
        try!(writer.write_all(&self.file_signature));
        try!(writer.write_u16::<LittleEndian>(self.file_source_id));
        try!(writer.write_u16::<LittleEndian>(self.global_encoding));
        try!(writer.write_u32::<LittleEndian>(self.guid_data_1));
        try!(writer.write_u16::<LittleEndian>(self.guid_data_2));
        try!(writer.write_u16::<LittleEndian>(self.guid_data_3));
        try!(writer.write_all(&self.guid_data_4));
        try!(writer.write_u8(self.version_major));
        try!(writer.write_u8(self.version_minor));
        try!(writer.write_all(&self.system_identifier));
        try!(writer.write_all(&self.generating_software));
        try!(writer.write_u16::<LittleEndian>(self.file_creation_day_of_year));
        try!(writer.write_u16::<LittleEndian>(self.file_creation_year));
        try!(writer.write_u16::<LittleEndian>(self.header_size));
        try!(writer.write_u32::<LittleEndian>(self.offset_to_point_data));
        try!(writer.write_u32::<LittleEndian>(self.number_of_variable_length_records));
        try!(writer.write_u8(self.point_data_format.0));
        try!(writer.write_u16::<LittleEndian>(self.point_data_record_length));
        try!(writer.write_u32::<LittleEndian>(self.number_of_point_records));
        for n in &self.number_of_points_by_return {
            try!(writer.write_u32::<LittleEndian>(*n));
        }
        try!(writer.write_f64::<LittleEndian>(self.x_scale_factor));
        try!(writer.write_f64::<LittleEndian>(self.y_scale_factor));
        try!(writer.write_f64::<LittleEndian>(self.z_scale_factor));
        try!(writer.write_f64::<LittleEndian>(self.x_offset));
        try!(writer.write_f64::<LittleEndian>(self.y_offset));
        try!(writer.write_f64::<LittleEndian>(self.z_offset));
        try!(writer.write_f64::<LittleEndian>(self.x_max));
        try!(writer.write_f64::<LittleEndian>(self.x_min));
        try!(writer.write_f64::<LittleEndian>(self.y_max));
        try!(writer.write_f64::<LittleEndian>(self.y_min));
        try!(writer.write_f64::<LittleEndian>(self.z_max));
        try!(writer.write_f64::<LittleEndian>(self.z_min));
        Ok(DEFAULT_BYTES_IN_HEADER)
    }

    /// Calculates the size of this header and assigns it to the header structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::header::Header;
    /// let mut header = Header::new();
    /// header.calculate_size();
    /// assert_eq!(227, header.header_size);
    /// ```
    pub fn calculate_size(&mut self) {
        self.header_size = DEFAULT_BYTES_IN_HEADER
    }
}

/// A wrapper around the u8 of a point data format.
///
/// Formats have powers, so we encapsulate that through this struct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointDataFormat(u8);

impl PointDataFormat {
    /// Creates a point data format for this u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::header::PointDataFormat;
    /// assert!(PointDataFormat::from_u8(0).is_ok());
    /// assert!(PointDataFormat::from_u8(1).is_ok());
    /// assert!(PointDataFormat::from_u8(127).is_err());
    /// ```
    pub fn from_u8(n: u8) -> Result<PointDataFormat> {
        if n < 4 {
            Ok(PointDataFormat(n))
        } else {
            Err(Error::InvalidPointDataFormat(n))
        }
    }

    /// Returns true if this point data format has time.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::header::PointDataFormat;
    /// assert!(!PointDataFormat::from_u8(0).unwrap().has_time());
    /// assert!(PointDataFormat::from_u8(1).unwrap().has_time());
    /// ```
    pub fn has_time(&self) -> bool {
        self.0 == 1 || self.0 == 3
    }

    /// Returns true if this point data format has color.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::header::PointDataFormat;
    /// assert!(!PointDataFormat::from_u8(0).unwrap().has_color());
    /// assert!(PointDataFormat::from_u8(2).unwrap().has_color());
    /// ```
    pub fn has_color(&self) -> bool {
        self.0 == 2 || self.0 == 3
    }
}

impl fmt::Display for PointDataFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
