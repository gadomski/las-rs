//! Raw file metadata.

use {Result, Version};
use byteorder::{LittleEndian, ReadBytesExt};
use feature::{Evlrs, LargeFiles, Waveforms};
use std::io::{Read, Write};

const IS_COMPRESSED_MASK: u8 = 0x80;

/// A las header.
///
/// The documentation for each member is taken directly from the las 1.2 spec, except in cases
/// where the field's usage has changed, in which case notes about its uses over versions are
/// included, with particular attention paid to the latest version, 1.4.
#[derive(Debug, PartialEq)]
pub struct Header {
    /// The file signature must contain the four characters “LASF”, and it is required by the LAS
    /// specification.
    ///
    /// These four characters can be checked by user software as a quick look
    /// initial determination of file type.
    pub file_signature: [u8; 4],

    /// This field should be set to a value between 1 and 65,535, inclusive.
    ///
    /// A value of zero (0) is interpreted to mean that an ID has not been assigned. In this case,
    /// processing software is free to assign any valid number. Note that this scheme allows a
    /// LIDAR project to contain up to 65,535 unique sources. A source can be considered an
    /// original flight line or it can be the result of merge and/or extract operations.
    pub file_source_id: u16,

    /// This is a bit field used to indicate certain global properties about the file.
    ///
    /// In LAS 1.2 (the version in which this field was introduced), only the low bit is defined
    /// (this is the bit, that if set, would have the unsigned integer yield a value of 1).
    ///
    /// | Bits | Field name | las versions | Description |
    /// | ---- | ---------- | ------------ | ----------- |
    /// | 0 | GPS time type | 1.2 and above | The meaning of GPS Time in the point records. If this bit is not set, the GPS time in the point record fields is GPS Week Time (the same as versions 1.0 through 1.2 of LAS).  Otherwise, if this bit is set, the GPS Time is standard GPS Time (satellite GPS Time) minus 1 x 109 (Adjusted Standard GPS Time). The offset moves the time back to near zero to improve floating point resolution. |
    /// | 1 | Waveform data packets internal | 1.3 | If this bit is set, the waveform data packets are located within this file (note that this bit is mutually exclusive with bit 2). This is deprecated in las 1.4.
    /// | 2 | Waveform Data Packets External | 1.3 and above | If this bit is set, the waveform data packets are located externally in an auxiliary file with the same base name as this file but the extension *.wdp. (note that this bit is mutually exclusive with bit 1) |
    /// | 3 | Return numbers have been synthetically generated | 1.3 and above | If this bit is set, the point return numbers in the point data records have been synthetically generated. This could be the case, for example, when a composite file is created by combining a First Return File and a Last Return File. In this case, first return data will be labeled “1 of 2” and second return data will be labeled "2 of 2" |
    /// | 4 | WKT | 1.4 | If set, the Coordinate Reference System (CRS) is WKT. If not set, the CRS is GeoTIFF. It should not be set if the file writer wishes to ensure legacy compatibility (which means the CRS must be GeoTIFF) |
    /// | 5:15 | Reserved | All | Must be set to zero |
    pub global_encoding: u16,

    /// The four fields that comprise a complete Globally Unique Identifier (GUID) are now reserved
    /// for use as a Project Identifier (Project ID).
    ///
    /// The field remains optional. The time of assignment of the Project ID is at the discretion
    /// of processing software. The Project ID should be the same for all files that are associated
    /// with a unique project. By assigning a Project ID and using a File Source ID (defined above)
    /// every file within a project and every point within a file can be uniquely identified,
    /// globally.
    pub guid: [u8; 16],

    /// The version number consists of a major and minor field.
    ///
    /// The major and minor fields combine to form the number that indicates the format number of
    /// the current specification itself. For example, specification number 1.2 (this version)
    /// would contain 1 in the major field and 2 in the minor field.
    pub version: Version,

    /// The version 1.0 specification assumes that LAS files are exclusively generated as a result
    /// of collection by a hardware sensor. Version 1.1 recognizes that files often result from
    /// extraction, merging or modifying existing data files.
    ///
    /// | Generating agent | System id |
    /// | ---------------- | --------- |
    /// | Hardware system | String identifying hardware (e.g. “ALTM 1210” or “ALS50” |
    /// | Merge of one or more files | "MERGE" |
    /// | Modification of a single file | "MODIFICATION" |
    /// | Extraction from one or more files | "EXTRACTION" |
    /// | Reprojection, rescaling, warping, etc. | "TRANSFORMATION" |
    /// | Some other operation | "OTHER" or a string up to 32 characters identifying the operation|
    pub system_identifier: [u8; 32],

    /// This information is ASCII data describing the generating software itself.
    ///
    /// This field provides a mechanism for specifying which generating software package and
    /// version was used during LAS file creation (e.g. “TerraScan V-10.8”, “REALM V-4.2” and
    /// etc.). If the character data is less than 16 characters, the remaining data must be null.
    pub generating_software: [u8; 32],

    /// Day, expressed as an unsigned short, on which this file was created.
    ///
    /// Day is computed as the Greenwich Mean Time (GMT) day. January 1 is considered day 1.
    pub file_creation_day_of_year: u16,

    /// The year, expressed as a four digit number, in which the file was created.
    pub file_creation_year: u16,

    /// The size, in bytes, of the Public Header Block itself.
    ///
    /// In the event that the header is extended by a software application through the addition of
    /// data at the end of the header, the Header Size field must be updated with the new header
    /// size. Extension of the Public Header Block is discouraged; the Variable Length Records
    /// should be used whenever possible to add custom header data. In the event a generating
    /// software package adds data to the Public Header Block, this data must be placed at the end
    /// of the structure and the Header Size must be updated to reflect the new size.
    ///
    /// # las 1.4
    ///
    /// For LAS 1.4 this size is 375 bytes. In the event that the header is extended by a new
    /// revision of the LAS specification through the addition of data at the end of the header,
    /// the Header Size field will be updated with the new header size. The Public Header Block may
    /// not be extended by users.
    pub header_size: u16,

    /// The actual number of bytes from the beginning of the file to the first field of the first
    /// point record data field.
    ///
    /// This data offset must be updated if any software adds data from the Public Header Block or
    /// adds/removes data to/from the Variable Length Records.
    pub offset_to_point_data: u32,

    /// This field contains the current number of Variable Length Records.
    ///
    /// This number must be updated if the number of Variable Length Records changes at any time.
    pub number_of_variable_length_records: u32,

    /// The point data format ID corresponds to the point data record format type.
    ///
    /// LAS 1.2 defines types 0, 1, 2 and 3. LAS 1.4 defines types 0 through 10.
    pub point_data_format_id: u8,

    /// The size, in bytes, of the Point Data Record.
    pub point_data_record_length: u16,

    /// This field contains the total number of point records within the file.
    pub number_of_point_records: u32,

    /// This field contains an array of the total point records per return.
    ///
    /// The first unsigned long value will be the total number of records from the first return,
    /// and the second contains the total number for return two, and so forth up to five returns.
    pub number_of_points_by_return: [u32; 5],

    /// The scale factor fields contain a double floating point value that is used to scale the
    /// corresponding X, Y, and Z long values within the point records.
    ///
    /// The corresponding X, Y, and Z scale factor must be multiplied by the X, Y, or Z point
    /// record value to get the actual X, Y, or Z coordinate. For example, if the X, Y, and Z
    /// coordinates are intended to have two decimal point values, then each scale factor will
    /// contain the number 0.01.
    pub x_scale_factor: f64,
    #[allow(missing_docs)]
    pub y_scale_factor: f64,
    #[allow(missing_docs)]
    pub z_scale_factor: f64,

    /// The offset fields should be used to set the overall offset for the point records.
    ///
    /// In general these numbers will be zero, but for certain cases the resolution of the point
    /// data may not be large enough for a given projection system. However, it should always be
    /// assumed that these numbers are used. So to scale a given X from the point record, take the
    /// point record X multiplied by the X scale factor, and then add the X offset.
    ///
    /// Xcoordinate = (Xrecord * Xscale) + Xoffset
    ///
    /// Ycoordinate = (Yrecord * Yscale) + Yoffset
    ///
    /// Zcoordinate = (Zrecord * Zscale) + Zoffset
    pub x_offset: f64,
    #[allow(missing_docs)]
    pub y_offset: f64,
    #[allow(missing_docs)]
    pub z_offset: f64,

    /// The max and min data fields are the actual unscaled extents of the LAS point file data,
    /// specified in the coordinate system of the LAS data.
    pub max_x: f64,
    #[allow(missing_docs)]
    pub min_x: f64,
    #[allow(missing_docs)]
    pub max_y: f64,
    #[allow(missing_docs)]
    pub min_y: f64,
    #[allow(missing_docs)]
    pub max_z: f64,
    #[allow(missing_docs)]
    pub min_z: f64,

    /// **las 1.3 and 1.4**: This value provides the offset, in bytes, from the beginning of the
    /// LAS file to the first byte of the Waveform Data Package Record.
    ///
    /// Note that this will be the first byte of the Waveform Data Packet header. If no waveform
    /// records are contained within the file, this value must be zero. It should be noted that LAS
    /// 1.4 allows multiple Extended Variable Length Records (EVLR) and that the Waveform Data
    /// Packet Record is not necessarily the first EVLR in the file.
    pub start_of_waveform_data_packet_record: Option<u64>,

    #[allow(missing_docs)]
    pub evlr: Option<Evlr>,

    #[allow(missing_docs)]
    pub large_file: Option<LargeFile>,

    #[allow(missing_docs)]
    pub padding: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(missing_docs)]
pub struct Evlr {
    /// **las 1.4**: This value provides the offset, in bytes, from the beginning of the LAS file to the first
    /// byte of the first EVLR.
    pub start_of_first_evlr: u64,

    /// **las 1.4**: This field contains the current number of EVLRs (including, if present, the Waveform Data
    /// Packet Record) that are stored in the file after the Point Data Records. This number must
    /// be updated if the number of EVLRs changes. If there are no EVLRs this value is zero.
    pub number_of_evlrs: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(missing_docs)]
pub struct LargeFile {
    /// **las 1.4**: This field contains the total number of point records in the file.
    ///
    /// Note that this field must always be correctly populated, regardless of legacy mode intent.
    pub number_of_point_records: u64,

    /// **las 1.4**: These fields contain an array of the total point records per return.
    ///
    /// The first value will be the total number of records from the first return, the second
    /// contains the total number for return two, and so on up to fifteen returns. Note that these
    /// fields must always be correctly populated, regardless of legacy mode intent.
    pub number_of_points_by_return: [u64; 15],
}

impl Header {
    /// Reads a raw header from a `Read`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use las::raw::Header;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// let header = Header::read_from(&mut file).unwrap();
    /// ```
    pub fn read_from<R: Read>(mut read: R) -> Result<Header> {
        use utils;

        let mut file_signature = [0; 4];
        read.read_exact(&mut file_signature)?;
        let file_source_id = read.read_u16::<LittleEndian>()?;
        let global_encoding = read.read_u16::<LittleEndian>()?;
        let mut guid = [0; 16];
        read.read_exact(&mut guid)?;
        let version_major = read.read_u8()?;
        let version_minor = read.read_u8()?;
        let version = Version::new(version_major, version_minor);
        let mut system_identifier = [0; 32];
        read.read_exact(&mut system_identifier)?;
        let mut generating_software = [0; 32];
        read.read_exact(&mut generating_software)?;
        let file_creation_day_of_year = read.read_u16::<LittleEndian>()?;
        let file_creation_year = read.read_u16::<LittleEndian>()?;
        let header_size = read.read_u16::<LittleEndian>()?;
        let offset_to_point_data = read.read_u32::<LittleEndian>()?;
        let number_of_variable_length_records = read.read_u32::<LittleEndian>()?;
        let point_data_format_id = read.read_u8()?;
        let point_data_record_length = read.read_u16::<LittleEndian>()?;
        let number_of_point_records = read.read_u32::<LittleEndian>()?;
        let mut number_of_points_by_return = [0; 5];
        for n in number_of_points_by_return.iter_mut() {
            *n = read.read_u32::<LittleEndian>()?;
        }
        let x_scale_factor = read.read_f64::<LittleEndian>()?;
        let y_scale_factor = read.read_f64::<LittleEndian>()?;
        let z_scale_factor = read.read_f64::<LittleEndian>()?;
        let x_offset = read.read_f64::<LittleEndian>()?;
        let y_offset = read.read_f64::<LittleEndian>()?;
        let z_offset = read.read_f64::<LittleEndian>()?;
        let max_x = read.read_f64::<LittleEndian>()?;
        let min_x = read.read_f64::<LittleEndian>()?;
        let max_y = read.read_f64::<LittleEndian>()?;
        let min_y = read.read_f64::<LittleEndian>()?;
        let max_z = read.read_f64::<LittleEndian>()?;
        let min_z = read.read_f64::<LittleEndian>()?;
        let start_of_waveform_data_packet_record = if version.supports::<Waveforms>() {
            utils::some_or_none_if_zero(read.read_u64::<LittleEndian>()?)
        } else {
            None
        };
        let evlr = if version.supports::<Evlrs>() {
            Evlr::read_from(&mut read)?.into_option()
        } else {
            None
        };
        let large_file = if version.supports::<LargeFiles>() {
            Some(LargeFile::read_from(&mut read)?)
        } else {
            None
        };
        let padding = if header_size > version.header_size() {
            let mut bytes = vec![0; (header_size - version.header_size()) as usize];
            read.read_exact(&mut bytes)?;
            bytes
        } else {
            Vec::new()
        };
        Ok(Header {
            file_signature: file_signature,
            file_source_id: file_source_id,
            global_encoding: global_encoding,
            guid: guid,
            version: version,
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
            start_of_waveform_data_packet_record: start_of_waveform_data_packet_record,
            evlr: evlr,
            large_file: large_file,
            padding: padding,
        })
    }

    /// Returns true if this raw header is for compressed las data.
    ///
    /// Though this isn't part of the las spec, the two high bits of the point data format id have
    /// been used to indicate compressed data, though only the high bit is currently used.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::Header;
    /// let mut header = Header::default();
    /// assert!(!header.is_compressed());
    /// header.point_data_format_id = 131;
    /// assert!(header.is_compressed());
    /// ```
    pub fn is_compressed(&self) -> bool {
        (self.point_data_format_id & IS_COMPRESSED_MASK) == IS_COMPRESSED_MASK
    }

    /// Writes a raw header to a `Write`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::raw::Header;
    /// let mut cursor = Cursor::new(Vec::new());
    /// let header = Header::default();
    /// header.write_to(&mut cursor).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W) -> Result<()> {
        use byteorder::WriteBytesExt;

        write.write_all(&self.file_signature)?;
        write.write_u16::<LittleEndian>(self.file_source_id)?;
        write.write_u16::<LittleEndian>(self.global_encoding)?;
        write.write_all(&self.guid)?;
        write.write_u8(self.version.major)?;
        write.write_u8(self.version.minor)?;
        write.write_all(&self.system_identifier)?;
        write.write_all(&self.generating_software)?;
        write.write_u16::<LittleEndian>(
            self.file_creation_day_of_year,
        )?;
        write.write_u16::<LittleEndian>(self.file_creation_year)?;
        write.write_u16::<LittleEndian>(self.header_size)?;
        write.write_u32::<LittleEndian>(self.offset_to_point_data)?;
        write.write_u32::<LittleEndian>(
            self.number_of_variable_length_records,
        )?;
        write.write_u8(self.point_data_format_id)?;
        write.write_u16::<LittleEndian>(
            self.point_data_record_length,
        )?;
        write.write_u32::<LittleEndian>(
            self.number_of_point_records,
        )?;
        for n in self.number_of_points_by_return.iter() {
            write.write_u32::<LittleEndian>(*n)?;
        }
        write.write_f64::<LittleEndian>(self.x_scale_factor)?;
        write.write_f64::<LittleEndian>(self.y_scale_factor)?;
        write.write_f64::<LittleEndian>(self.z_scale_factor)?;
        write.write_f64::<LittleEndian>(self.x_offset)?;
        write.write_f64::<LittleEndian>(self.y_offset)?;
        write.write_f64::<LittleEndian>(self.z_offset)?;
        write.write_f64::<LittleEndian>(self.max_x)?;
        write.write_f64::<LittleEndian>(self.min_x)?;
        write.write_f64::<LittleEndian>(self.max_y)?;
        write.write_f64::<LittleEndian>(self.min_y)?;
        write.write_f64::<LittleEndian>(self.max_z)?;
        write.write_f64::<LittleEndian>(self.min_z)?;
        if self.version.supports::<Waveforms>() {
            write.write_u64::<LittleEndian>(
                self.start_of_waveform_data_packet_record.unwrap_or(0),
            )?;
        }
        if self.version.supports::<Evlrs>() {
            let elvr = self.evlr.unwrap_or_else(Evlr::default);
            write.write_u64::<LittleEndian>(elvr.start_of_first_evlr)?;
            write.write_u32::<LittleEndian>(elvr.number_of_evlrs)?;
        }
        if self.version.supports::<LargeFiles>() {
            let large_file = self.large_file.unwrap_or_else(LargeFile::default);
            write.write_u64::<LittleEndian>(
                large_file.number_of_point_records,
            )?;
            for n in &large_file.number_of_points_by_return {
                write.write_u64::<LittleEndian>(*n)?;
            }
        }
        if !self.padding.is_empty() {
            write.write_all(&self.padding)?;
        }
        Ok(())
    }
}

impl Default for Header {
    fn default() -> Header {
        let version = Version::new(1, 2);
        Header {
            file_signature: ::raw::LASF,
            file_source_id: 0,
            global_encoding: 0,
            guid: [0; 16],
            version: version,
            system_identifier: [0; 32],
            generating_software: [0; 32],
            file_creation_day_of_year: 0,
            file_creation_year: 0,
            header_size: version.header_size(),
            offset_to_point_data: version.header_size() as u32,
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
            start_of_waveform_data_packet_record: None,
            evlr: None,
            large_file: None,
            padding: Vec::new(),
        }
    }
}

impl Evlr {
    fn read_from<R: Read>(mut read: R) -> Result<Evlr> {
        Ok(Evlr {
            start_of_first_evlr: read.read_u64::<LittleEndian>()?,
            number_of_evlrs: read.read_u32::<LittleEndian>()?,
        })
    }

    fn into_option(self) -> Option<Evlr> {
        if self.start_of_first_evlr == 0 && self.number_of_evlrs == 0 {
            None
        } else {
            Some(self)
        }
    }
}

impl LargeFile {
    fn read_from<R: Read>(mut read: R) -> Result<LargeFile> {
        let number_of_point_records = read.read_u64::<LittleEndian>()?;
        let mut number_of_points_by_return = [0; 15];
        for n in &mut number_of_points_by_return {
            *n = read.read_u64::<LittleEndian>()?
        }
        Ok(LargeFile {
            number_of_point_records: number_of_point_records,
            number_of_points_by_return: number_of_points_by_return,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_compressed() {
        let mut header = Header {
            point_data_format_id: 0,
            ..Default::default()
        };
        assert!(!header.is_compressed());
        header.point_data_format_id = 131;
        assert!(header.is_compressed());
    }

    macro_rules! roundtrip {
        ($name:ident, $minor:expr) => {
            mod $name {
                #[test]
                fn roundtrip() {
                    use std::io::Cursor;
                    use super::*;

                    let version = Version::new(1, $minor);
                    let mut header = Header {
                        version: version,
                        ..Default::default()
                    };
                    if version.minor == 4 {
                        header.large_file = Some(LargeFile::default());
                    }
                    let mut cursor = Cursor::new(Vec::new());
                    header.write_to(&mut cursor).unwrap();
                    cursor.set_position(0);
                    assert_eq!(header, Header::read_from(cursor).unwrap());
                }
            }
        }
    }

    roundtrip!(las_1_0, 0);
    roundtrip!(las_1_1, 1);
    roundtrip!(las_1_2, 2);
    roundtrip!(las_1_3, 3);
    roundtrip!(las_1_4, 4);
}
