//! Las headers.

use std::io::Read;

use byteorder::ReadBytesExt;
use byteorder::LittleEndian;
use rustc_serialize::hex::ToHex;

use Error;
use Result;

/// Reads n bytes into a buffer, or returns a `ReadError`.
macro_rules! try_read_n {
    ($reader:expr, $data:expr, $n:expr) => {{
        let took = try!($reader.take($n).read(&mut $data));
        if took != $n {
            return Err(Error::ReadError(format!("Tried to take {} bytes, only took {}", $n, took)));
        }
    }};
}

/// Converts a `u8` buffer to a string, using null bytes as the terminator.
///
/// Also checks for characters after the first null byte, which is invalid according to the las
/// spec.
fn buffer_to_string(buffer: &[u8]) -> Result<String> {
    let mut s = String::with_capacity(buffer.len());
    let mut seen_null_byte = false;
    for &byte in buffer {
        if byte > 0u8 {
            if seen_null_byte {
                return Err(Error::CharacterAfterNullByte);
            } else {
                s.push(byte as char);
            }
        } else {
            seen_null_byte = true;
        }
    }
    Ok(s)
}

/// Three f64 values in x, y, z order.
#[derive(Debug, Default)]
pub struct Triplet {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Project ID newtype.
pub struct ProjectId([u8; 4], [u8; 2], [u8; 2], [u8; 8]);

impl ProjectId {
    /// Returns a hexadecimal string representation of the project ID.
    ///
    /// The format for this string was derived from libLAS's output.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::header::ProjectId;
    /// let project_id = ProjectId([0; 16]);
    /// assert_eq!("00000000-0000-0000-0000-000000000000", project_id.as_hex());
    /// ```
    pub fn as_hex(&self) -> String {
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
    pub scale_factors: Triplet,
    pub offsets: Triplet,
    pub mins: Triplet,
    pub maxs: Triplet,
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
            scale_factors: Default::default(),
            offsets: Default::default(),
            mins: Default::default(),
            maxs: Default::default(),
        }
    }
}

impl Header {
    /// Creates a new header from a `Read` object.
    ///
    /// # Examples
    ///
    /// ```
    /// let reader = std::fs::File::open("data/1.2_0.las").unwrap();
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
        let mut buffer = [0u8; 32];
        try_read_n!(reader, buffer, 32);
        header.system_identifier = try!(buffer_to_string(&buffer));
        try_read_n!(reader, buffer, 32);
        header.generating_software = try!(buffer_to_string(&buffer));
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
        header.scale_factors.x = try!(reader.read_f64::<LittleEndian>());
        header.scale_factors.y = try!(reader.read_f64::<LittleEndian>());
        header.scale_factors.z = try!(reader.read_f64::<LittleEndian>());
        header.offsets.x = try!(reader.read_f64::<LittleEndian>());
        header.offsets.y = try!(reader.read_f64::<LittleEndian>());
        header.offsets.z = try!(reader.read_f64::<LittleEndian>());
        header.maxs.x = try!(reader.read_f64::<LittleEndian>());
        header.mins.x = try!(reader.read_f64::<LittleEndian>());
        header.maxs.y = try!(reader.read_f64::<LittleEndian>());
        header.mins.y = try!(reader.read_f64::<LittleEndian>());
        header.maxs.z = try!(reader.read_f64::<LittleEndian>());
        header.mins.z = try!(reader.read_f64::<LittleEndian>());
        Ok(header)
    }
}
