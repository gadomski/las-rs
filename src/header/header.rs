use {Bounds, Transform, Vector, Vlr};
use chrono::{Date, UTC};
use header::{GpsTimeType, HEADER_SIZE};
use point::Format;

/// Metadata describing the layout, source, and interpretation of the points.
///
/// This structure is a higher-level representation of the header data, but can be converted into a
/// `RawHeader` to write actual LAS headers.
#[derive(Clone, Debug)]
pub struct Header {
    /// A project-wide unique ID for the file.
    pub file_source_id: u16,
    /// The time type for GPS time.
    pub gps_time_type: GpsTimeType,
    /// Optional globally-unique identifier.
    pub guid: [u8; 16],
    /// The LAS version of this file.
    pub version: (u8, u8),
    /// The system that produced this file.
    ///
    /// If hardware, this should be the name of the hardware. Otherwise, maybe describe the
    /// operation performed to create these data?
    pub system_identifier: String,
    /// The software which generated these data.
    pub generating_software: String,
    /// The date these data were collected.
    ///
    /// If the date in the header was crap, this is `None`.
    pub date: Option<Date<UTC>>,
    /// Optional and discouraged padding between the header and the `Vlr`s.
    pub padding: Vec<u8>,
    /// Optional and discouraged padding between the `Vlr`s and the points.
    pub vlr_padding: Vec<u8>,
    /// The `point::Format` of these points.
    ///
    /// TODO extra bytes.
    pub point_format: Format,
    /// The three `Transform`s used to convert xyz coordinates from floats to signed integers.
    ///
    /// This is how you specify scales and offsets.
    pub transforms: Vector<Transform>,
    /// The bounds of these LAS data.
    pub bounds: Bounds,
    /// The number of points.
    pub number_of_points: u32,
    /// The number of points of each return type (1-5).
    pub number_of_points_by_return: [u32; 5],
    /// Variable length records.
    pub vlrs: Vec<Vlr>,
}

impl Header {
    /// Returns the length of this header.
    ///
    /// This includes both the length of the written header and the padding.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Header;
    /// let header = Header { ..Default::default() };
    /// assert_eq!(227, header.len());
    /// ```
    pub fn len(&self) -> u16 {
        HEADER_SIZE + self.padding.len() as u16
    }
}

impl Default for Header {
    fn default() -> Header {
        Header {
            file_source_id: 0,
            gps_time_type: GpsTimeType::Week,
            bounds: Default::default(),
            date: Some(UTC::today()),
            generating_software: format!("las-rs {}", env!("CARGO_PKG_VERSION")),
            guid: Default::default(),
            number_of_points: 0,
            number_of_points_by_return: [0; 5],
            padding: Vec::new(),
            vlr_padding: Vec::new(),
            point_format: 0.into(),
            system_identifier: "las-rs".to_string(),
            transforms: Default::default(),
            version: (1, 2),
            vlrs: Vec::new(),
        }
    }
}
