//! Raw structures for las entities.
//!
//! These structures map directly onto the structures as defined in the las formats.

mod point;
mod vlr;
mod header;

pub use self::header::{HEADER_SIZE, Header};
pub use self::point::Point;
pub use self::vlr::Vlr;

/// The file magic number used for all las files.
pub const LASF: [u8; 4] = *b"LASF";

/// The point data start signature required by las 1.0.
pub const POINT_DATA_START_SIGNATURE: [u8; 2] = [0xDD, 0xCC];
