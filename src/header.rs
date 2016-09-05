use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::{Date, TimeZone, UTC};

use {Error, Result};
use global_encoding::GlobalEncoding;
use point::Format;
use utils::{Bounds, Triple};
use version::Version;
use vlr::Vlr;

const HEADER_SIZE: u16 = 227;
const DEFAULT_SYSTEM_ID: [u8; 32] = [108, 97, 115, 45, 114, 115, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

/// The LAS header.
#[derive(Clone, Debug)]
pub struct Header {
    /// The file source ID.
    ///
    /// This does not exist for LAS 1.0 files.
    pub file_source_id: Option<u16>,
    /// The global encoding.
    ///
    /// This does not exist for LAS 1.1 and 1.0 files.
    pub global_encoding: Option<GlobalEncoding>,
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
    pub offset_to_data: u32,
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
    /// The scaling that is applied to points as they are read.
    pub scale: Triple<f64>,
    /// The offset of the points, in each dimension.
    pub offset: Triple<f64>,
    /// The three-dimensional bounds, from the header.
    pub bounds: Bounds<f64>,
    /// Variable length records.
    pub vlrs: Vec<Vlr>,
    /// Arbitrary byte padding between the header + VLRs and the points.
    pub padding: u32,
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
            file_source_id: Some(0),
            global_encoding: Some(Default::default()),
            project_id: Default::default(),
            version: Version::new(1, 2),
            system_id: DEFAULT_SYSTEM_ID,
            generating_software: generating_software,
            header_size: HEADER_SIZE,
            offset_to_data: HEADER_SIZE as u32,
            file_creation_date: UTC::today(),
            point_format: format,
            extra_bytes: 0,
            point_count: 0,
            point_count_by_return: Default::default(),
            scale: Triple::new(1., 1., 1.),
            offset: Triple::new(0., 0., 0.),
            bounds: Default::default(),
            vlrs: Vec::new(),
            padding: 0,
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

        if !version.has_file_source_id() && file_source_id != 0 {
            return Err(Error::ReservedIsNotZero);
        }
        // TODO make reading a header less error-ful
        let file_source_id = if version.has_file_source_id() {
            Some(file_source_id)
        } else if file_source_id == 0 {
            None
        } else {
            return Err(Error::ReservedIsNotZero);
        };
        // TODO make reading a header less error-ful
        let global_encoding = if version.has_global_encoding() {
            Some(GlobalEncoding::from(global_encoding))
        } else if global_encoding == 0 {
            None
        } else {
            return Err(Error::ReservedIsNotZero);
        };

        let mut system_id = [0; 32];
        try!(self.read_exact(&mut system_id));
        let mut generating_software = [0; 32];
        try!(self.read_exact(&mut generating_software));
        let day = try!(self.read_u16::<LittleEndian>());
        let year = try!(self.read_u16::<LittleEndian>());
        let file_creation_date = UTC.yo(year as i32, day as u32);
        let header_size = try!(self.read_u16::<LittleEndian>());
        let offset_to_data = try!(self.read_u32::<LittleEndian>());
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
        // TODO mush scale and offset together
        let scale = Triple {
            x: try!(self.read_f64::<LittleEndian>()),
            y: try!(self.read_f64::<LittleEndian>()),
            z: try!(self.read_f64::<LittleEndian>()),
        };
        let offset = Triple {
            x: try!(self.read_f64::<LittleEndian>()),
            y: try!(self.read_f64::<LittleEndian>()),
            z: try!(self.read_f64::<LittleEndian>()),
        };
        let maxx = try!(self.read_f64::<LittleEndian>());
        let minx = try!(self.read_f64::<LittleEndian>());
        let maxy = try!(self.read_f64::<LittleEndian>());
        let miny = try!(self.read_f64::<LittleEndian>());
        let maxz = try!(self.read_f64::<LittleEndian>());
        let minz = try!(self.read_f64::<LittleEndian>());
        let bounds = Bounds::new(minx, miny, minz, maxx, maxy, maxz);

        // TODO read VLRs seperately
        let vlrs = try!((0..num_vlrs)
            .map(|_| {
                let mut vlr: Vlr = Default::default();
                try!(self.read_u16::<LittleEndian>()); // reserved
                try!(self.read_exact(&mut vlr.user_id));
                vlr.record_id = try!(self.read_u16::<LittleEndian>());
                vlr.record_length = try!(self.read_u16::<LittleEndian>());
                try!(self.read_exact(&mut vlr.description));
                try!(self.take(vlr.record_length as u64).read_to_end(&mut vlr.data));
                Ok(vlr)
            })
            .collect::<Result<Vec<Vlr>>>());
        Ok(Header {
            file_source_id: file_source_id,
            global_encoding: global_encoding,
            project_id: project_id,
            version: version,
            system_id: system_id,
            generating_software: generating_software,
            header_size: header_size,
            offset_to_data: offset_to_data,
            file_creation_date: file_creation_date,
            point_format: point_format,
            extra_bytes: extra_bytes as u16,
            point_count: point_count,
            point_count_by_return: point_count_by_return,
            scale: scale,
            offset: offset,
            bounds: bounds,
            padding: offset_to_data -
                     vlrs.iter().fold(header_size as u32, |acc, vlr| acc + vlr.len()),
            vlrs: vlrs,
        })
    }
}
