use Result;
use std::io::{Read, Write};

/// The default size of a las header.
pub const HEADER_SIZE: u16 = 227;
const IS_COMPRESSED_MASK: u8 = 0x80;

/// A raw header that maps directly onto the file structure.
#[derive(Debug)]
#[allow(missing_docs)]
pub struct Header {
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
        use byteorder::{LittleEndian, ReadBytesExt};
        let mut file_signature = [0; 4];
        read.read_exact(&mut file_signature)?;
        let file_source_id = read.read_u16::<LittleEndian>()?;
        let global_encoding = read.read_u16::<LittleEndian>()?;
        let mut guid = [0; 16];
        read.read_exact(&mut guid)?;
        let version_major = read.read_u8()?;
        let version_minor = read.read_u8()?;
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
        // TODO las 1.4
        let padding = if header_size > HEADER_SIZE {
            let mut bytes = vec![0; (header_size - HEADER_SIZE) as usize];
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
        use byteorder::{LittleEndian, WriteBytesExt};
        write.write_all(&self.file_signature)?;
        write.write_u16::<LittleEndian>(self.file_source_id)?;
        write.write_u16::<LittleEndian>(self.global_encoding)?;
        write.write_all(&self.guid)?;
        write.write_u8(self.version_major)?;
        write.write_u8(self.version_minor)?;
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
        if !self.padding.is_empty() {
            write.write_all(&self.padding)?;
        }
        Ok(())
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            file_signature: ::raw::LASF,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn not_lasf() {
        assert!(Header::read_from(File::open("README.md").unwrap()).is_err());
    }

    // TODO test header size is valid
    // TODO writing without LASF

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
}
