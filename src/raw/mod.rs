//! Raw structures that map directly onto structures as defined in the las format specifications.
//!
//! In general, these structures are "dumb", meaning that they do the least amount of validity
//! checking. For example:
//!
//! ```
//! let raw_header = las::raw::Header::default();
//! assert_eq!((0, 0), raw_header.version);
//!
//! let header = las::Header::default();
//! assert_eq!(las::Version::new(1, 2), header.version);
//! ```
//!
//! In general, users should prefer to use the non-raw versions, e.g. `las::Header` over
//! `las::raw::Header`, in order to ensure that they are following The Rules.

mod point;
mod vlr;
mod header;

pub use self::header::Header;
pub use self::point::Point;
pub use self::vlr::Vlr;

/// The file magic number used for all las files.
pub const LASF: [u8; 4] = *b"LASF";

/// The point data start signature required by las 1.0.
pub const POINT_DATA_START_SIGNATURE: [u8; 2] = [0xDD, 0xCC];
