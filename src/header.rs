//! Las headers.

use std::io::Read;

use Error;
use Result;

macro_rules! try_read_n {
    ($reader:expr, $data:expr, $n:expr) => {{
        let took = try!($reader.take($n).read(&mut $data));
        if took != $n {
            return Err(Error::ReadError(format!("Tried to take {} bytes, only took {}", $n, took)));
        }
    }};
}

/// Three f64 values in x, y, z order.
#[derive(Debug, Default)]
pub struct Triplet {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub struct Header {
    pub file_signature: [u8; 4],
    pub file_source_id: u16,
    pub global_encoding: u16,
    pub project_id: [u8; 16],
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
            project_id: [0; 16],
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
    pub fn new<R: Read>(reader: R) -> Result<Header> {
        let mut header: Header = Default::default();
        let file_signature = try_read_n!(reader, header.file_signature, 4);
        Ok(header)
    }
}
