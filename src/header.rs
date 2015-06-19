//! Las headers.

use std::io::Read;

use byteorder::ReadBytesExt;
use byteorder::LittleEndian;
use rustc_serialize::hex::ToHex;

use Error;
use Result;
use io::LasStringExt;

/// Three f64 values in x, y, z order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Triplet {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Project ID newtype.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectId(pub [u8; 4], pub [u8; 2], pub [u8; 2], pub [u8; 8]);

impl ProjectId {
    /// Returns a hexadecimal string representation of the project ID.
    ///
    /// The format for this string was derived from libLAS's output.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::header::ProjectId;
    /// let project_id = ProjectId([0; 4], [0; 2], [0; 2], [0; 8]);
    /// assert_eq!("00000000-0000-0000-0000-000000000000", project_id.as_string());
    /// ```
    pub fn as_string(&self) -> String {
        let mut s = self.0.to_hex();
        s.push_str("-");
        s.push_str(&self.1.to_hex());
        s.push_str("-");
        s.push_str(&self.2.to_hex());
        s.push_str("-");
        s.push_str(&self.3[0..2].to_hex());
        s.push_str("-");
        s.push_str(&self.3[2..8].to_hex());
        s
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub file_signature: [u8; 4],
    pub file_source_id: u16,
    pub global_encoding: u16,
    pub project_id: ProjectId,
    pub version_major: u8,
    pub version_minor: u8,
    pub system_identifier: String,
    pub generating_software: String,
    pub file_creation_day_of_year: u16,
    pub file_creation_year: u16,
    pub header_size: u16,
    pub offset_to_point_data: u32,
    pub number_of_variable_length_records: u32,
    pub point_data_format_id: u8,
    pub point_data_record_length: u16,
    pub number_of_point_records: u32,
    pub number_of_points_by_return: [u32; 5],
    pub scale: Triplet,
    pub offset: Triplet,
    pub min: Triplet,
    pub max: Triplet,
}

impl Default for Header {
    fn default() -> Header {
        Header {
            file_signature: *b"LASF",
            file_source_id: 0,
            global_encoding: 0,
            project_id: ProjectId([0; 4], [0; 2], [0; 2], [0; 8]),
            version_major: 0,
            version_minor: 0,
            system_identifier: "".to_string(),
            generating_software: "".to_string(),
            file_creation_day_of_year: 0,
            file_creation_year: 0,
            header_size: 0,
            offset_to_point_data: 0,
            number_of_variable_length_records: 0,
            point_data_format_id: 0,
            point_data_record_length: 0,
            number_of_point_records: 0,
            number_of_points_by_return: [0; 5],
            scale: Default::default(),
            offset: Default::default(),
            min: Default::default(),
            max: Default::default(),
        }
    }
}

impl Header {
    /// Creates a new header from a `Read` object.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::header::Header;
    /// let mut reader = std::fs::File::open("data/1.2_0.las").unwrap();
    /// let header = Header::new(&mut reader);
    /// ```
    pub fn new<R: Read>(reader: &mut R) -> Result<Header> {
        let mut header: Header = Default::default();
        try_read_n!(reader, header.file_signature, 4);
        header.file_source_id = try!(reader.read_u16::<LittleEndian>());
        header.global_encoding = try!(reader.read_u16::<LittleEndian>());
        try_read_n!(reader, header.project_id.0, 4);
        try_read_n!(reader, header.project_id.1, 2);
        try_read_n!(reader, header.project_id.2, 2);
        try_read_n!(reader, header.project_id.3, 8);
        header.version_major = try!(reader.read_u8());
        header.version_minor = try!(reader.read_u8());
        header.system_identifier = try!(reader.read_las_string(32));
        header.generating_software = try!(reader.read_las_string(32));
        header.file_creation_day_of_year = try!(reader.read_u16::<LittleEndian>());
        header.file_creation_year = try!(reader.read_u16::<LittleEndian>());
        header.header_size = try!(reader.read_u16::<LittleEndian>());
        header.offset_to_point_data = try!(reader.read_u32::<LittleEndian>());
        header.number_of_variable_length_records = try!(reader.read_u32::<LittleEndian>());
        header.point_data_format_id = try!(reader.read_u8());
        header.point_data_record_length = try!(reader.read_u16::<LittleEndian>());
        header.number_of_point_records = try!(reader.read_u32::<LittleEndian>());
        header.number_of_points_by_return[0] = try!(reader.read_u32::<LittleEndian>());
        header.number_of_points_by_return[1] = try!(reader.read_u32::<LittleEndian>());
        header.number_of_points_by_return[2] = try!(reader.read_u32::<LittleEndian>());
        header.number_of_points_by_return[3] = try!(reader.read_u32::<LittleEndian>());
        header.number_of_points_by_return[4] = try!(reader.read_u32::<LittleEndian>());
        header.scale.x = try!(reader.read_f64::<LittleEndian>());
        header.scale.y = try!(reader.read_f64::<LittleEndian>());
        header.scale.z = try!(reader.read_f64::<LittleEndian>());
        header.offset.x = try!(reader.read_f64::<LittleEndian>());
        header.offset.y = try!(reader.read_f64::<LittleEndian>());
        header.offset.z = try!(reader.read_f64::<LittleEndian>());
        header.max.x = try!(reader.read_f64::<LittleEndian>());
        header.min.x = try!(reader.read_f64::<LittleEndian>());
        header.max.y = try!(reader.read_f64::<LittleEndian>());
        header.min.y = try!(reader.read_f64::<LittleEndian>());
        header.max.z = try!(reader.read_f64::<LittleEndian>());
        header.min.z = try!(reader.read_f64::<LittleEndian>());
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    #[test]
    fn header() {
        let mut reader = File::open("data/1.2_0.las").unwrap();
        let header = Header::new(&mut reader).unwrap();
        assert_eq!(*b"LASF", header.file_signature);
        assert_eq!(0, header.file_source_id);
        assert_eq!(0, header.global_encoding);
        assert_eq!("b8f18883-1baa-0841-bca3-6bc68e7b062e", header.project_id.as_string());
        assert_eq!(1, header.version_major);
        assert_eq!(2, header.version_minor);
        assert_eq!("libLAS", header.system_identifier);
        assert_eq!("libLAS 1.2", header.generating_software);
        assert_eq!(78, header.file_creation_day_of_year);
        assert_eq!(2008, header.file_creation_year);
        assert_eq!(227, header.header_size);
        assert_eq!(438, header.offset_to_point_data);
        assert_eq!(2, header.number_of_variable_length_records);
        assert_eq!(0, header.point_data_format_id);
        assert_eq!(20, header.point_data_record_length);
        assert_eq!(1, header.number_of_point_records);
        assert_eq!([0, 1, 0, 0, 0], header.number_of_points_by_return);
        assert_eq!(0.01, header.scale.x);
        assert_eq!(0.01, header.scale.y);
        assert_eq!(0.01, header.scale.z);
        assert_eq!(0.0, header.offset.x);
        assert_eq!(0.0, header.offset.y);
        assert_eq!(0.0, header.offset.z);
        assert_eq!(470692.447538, header.min.x);
        assert_eq!(4602888.904642, header.min.y);
        assert_eq!(16.0, header.min.z);
        assert_eq!(470692.447538, header.max.x);
        assert_eq!(4602888.904642, header.max.y);
        assert_eq!(16.0, header.max.z);
    }
}
