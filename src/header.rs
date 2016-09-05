use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{Date, TimeZone, UTC};

use {Error, Result};
use global_encoding::GlobalEncoding;
use point::Format;
use utils::{Bounds, LinearTransform, Triple};
use version::Version;

const HEADER_SIZE: u16 = 227;
const DEFAULT_SYSTEM_ID: [u8; 32] = [108, 97, 115, 45, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// The LAS header.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    /// The file source ID.
    ///
    /// This did not exist for LAS 1.0 files, but defaults to zero.
    pub file_source_id: u16,
    /// The global encoding.
    ///
    /// This did not exist for LAS 1.1 and 1.0 files.
    pub global_encoding: GlobalEncoding,
    /// The project id number.
    pub project_id: [u8; 16],
    /// The LAS version.
    pub version: Version,
    /// The system identifier.
    pub system_id: [u8; 32],
    /// The generating software.
    pub generating_software: [u8; 32],
    /// The size of the header.
    pub header_size: u16,
    /// The byte offset to the beginning of point data.
    pub offset_to_point_data: u32,
    /// The number of VLRs.
    pub num_vlrs: u32,
    /// The day of file creation.
    pub file_creation_date: Date<UTC>,
    /// The point format.
    pub point_format: Format,
    /// The number of extra bytes in the point beyond the standard.
    pub extra_bytes: u16,
    /// The number of points.
    ///
    /// This value is taken from the header and is notoriously inaccurate.
    pub point_count: u32,
    /// The number of points by return count.
    pub point_count_by_return: [u32; 5],
    /// The offsets and scaling that is applied to points as they are read.
    pub transforms: Triple<LinearTransform>,
    /// The three-dimensional bounds, from the header.
    pub bounds: Bounds<f64>,
}

impl Default for Header {
    fn default() -> Header {
        let format = Format::from(0);
        let generating_software_string = format!("las-rs {}", env!("CARGO_PKG_VERSION"));
        let mut generating_software = [0; 32];
        for (target, source) in generating_software.iter_mut()
            .zip(generating_software_string.bytes()) {
            *target = source;
        }
        Header {
            file_source_id: 0,
            global_encoding: Default::default(),
            project_id: Default::default(),
            version: Version::new(1, 2),
            system_id: DEFAULT_SYSTEM_ID,
            generating_software: generating_software,
            header_size: HEADER_SIZE,
            offset_to_point_data: HEADER_SIZE as u32,
            num_vlrs: 0,
            file_creation_date: UTC::today(),
            point_format: format,
            extra_bytes: 0,
            point_count: 0,
            point_count_by_return: Default::default(),
            transforms: Default::default(),
            bounds: Default::default(),
        }
    }
}

pub trait ReadHeader {
    fn read_header(&mut self) -> Result<Header>;
}

impl<R: Read> ReadHeader for R {
    fn read_header(&mut self) -> Result<Header> {
        let mut file_signature = String::new();
        try!(self.take(4).read_to_string(&mut file_signature));
        if file_signature != "LASF" {
            return Err(Error::InvalidFileSignature(file_signature));
        }
        let file_source_id = try!(self.read_u16::<LittleEndian>());
        let global_encoding = try!(self.read_u16::<LittleEndian>());
        let mut project_id = [0; 16];
        try!(self.read_exact(&mut project_id));
        let version = Version::new(try!(self.read_u8()), try!(self.read_u8()));
        let mut system_id = [0; 32];
        try!(self.read_exact(&mut system_id));
        let mut generating_software = [0; 32];
        try!(self.read_exact(&mut generating_software));
        let day = try!(self.read_u16::<LittleEndian>());
        let year = try!(self.read_u16::<LittleEndian>());
        let file_creation_date = UTC.yo(year as i32, day as u32);
        let header_size = try!(self.read_u16::<LittleEndian>());
        let offset_to_point_data = try!(self.read_u32::<LittleEndian>());
        let num_vlrs = try!(self.read_u32::<LittleEndian>());
        let point_format = Format::from(try!(self.read_u8()));
        // TODO make reading a header less error-ful
        if !point_format.is_supported() {
            return Err(Error::UnsupportedPointFormat(point_format));
        }
        let point_data_record_length = try!(self.read_u16::<LittleEndian>());
        let extra_bytes: i32 = point_data_record_length as i32 -
                               point_format.record_length() as i32;
        if extra_bytes < 0 {
            return Err(Error::InvalidPointDataRecordLength(point_format, point_data_record_length));
        }
        let point_count = try!(self.read_u32::<LittleEndian>());
        let mut point_count_by_return = [0; 5];
        for entry in point_count_by_return.iter_mut() {
            *entry = try!(self.read_u32::<LittleEndian>());
        }
        let scalex = try!(self.read_f64::<LittleEndian>());
        let scaley = try!(self.read_f64::<LittleEndian>());
        let scalez = try!(self.read_f64::<LittleEndian>());
        let offsetx = try!(self.read_f64::<LittleEndian>());
        let offsety = try!(self.read_f64::<LittleEndian>());
        let offsetz = try!(self.read_f64::<LittleEndian>());
        let transforms: Triple<LinearTransform> = Triple::new((scalex, offsetx).into(),
                                                              (scaley, offsety).into(),
                                                              (scalez, offsetz).into());
        let maxx = try!(self.read_f64::<LittleEndian>());
        let minx = try!(self.read_f64::<LittleEndian>());
        let maxy = try!(self.read_f64::<LittleEndian>());
        let miny = try!(self.read_f64::<LittleEndian>());
        let maxz = try!(self.read_f64::<LittleEndian>());
        let minz = try!(self.read_f64::<LittleEndian>());
        let bounds = Bounds::new(minx, miny, minz, maxx, maxy, maxz);
        Ok(Header {
            file_source_id: file_source_id,
            global_encoding: GlobalEncoding::from(global_encoding),
            project_id: project_id,
            version: version,
            system_id: system_id,
            generating_software: generating_software,
            header_size: header_size,
            offset_to_point_data: offset_to_point_data,
            num_vlrs: num_vlrs,
            file_creation_date: file_creation_date,
            point_format: point_format,
            extra_bytes: extra_bytes as u16,
            point_count: point_count,
            point_count_by_return: point_count_by_return,
            transforms: transforms,
            bounds: bounds,
        })
    }
}
