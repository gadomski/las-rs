//! Raw structures that map directly to their definitions in the las format specifications.
//!
//! In general, these structures are "dumb", meaning that they do the least amount of validity
//! checking possible without losing information. In general, users should prefer to use the
//! non-raw versions, e.g. `las::Header` over `las::raw::Header`, in order to ensure that they are
//! following The Rules. `into_raw` can be used to create the raw versions:
//!
//! ```
//! use las::{Vlr, Header, Point};
//! let raw_header = Header::default().into_raw().unwrap();
//! let raw_vlr = Vlr::default().into_raw(false).unwrap();
//! let raw_point = Point::default().into_raw(Default::default()).unwrap();
//! ```
//!
//! Raw structures all have `write_to` and `read_from` methods that can be used to put and extract
//! them from streams of bytes:
//!
//! ```
//! use las::point::Format;
//! use las::raw::{Header, Vlr, Point};
//! use std::io::Cursor;
//! let mut cursor = Cursor::new(Vec::new());
//! let point_format = Format::new(3).unwrap();
//!
//! // Write the structures in an arbitrary order.
//! Point::default().write_to(&mut cursor, point_format).unwrap();
//! Header::default().write_to(&mut cursor).unwrap();
//! Vlr::default().write_to(&mut cursor).unwrap();
//!
//! cursor.set_position(0);
//!
//! // And read them back.
//! Point::read_from(&mut cursor, point_format).unwrap();
//! Header::read_from(&mut cursor).unwrap();
//! Vlr::read_from(&mut cursor, false).unwrap();
//! ```

pub mod point;
pub mod vlr;
pub mod header;

pub use self::header::Header;
pub use self::point::Point;
pub use self::vlr::Vlr;

/// The file magic number used for all las files.
pub const LASF: [u8; 4] = *b"LASF";

/// The point data start signature required by las 1.0.
pub const POINT_DATA_START_SIGNATURE: [u8; 2] = [0xDD, 0xCC];
