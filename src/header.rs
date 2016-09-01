use chrono::{Date, UTC};

use global_encoding::GlobalEncoding;
use point::Format;
use utils::{Bounds, Triple};
use version::Version;
use vlr::Vlr;

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
