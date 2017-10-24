//! Variable length records.
//!
//! These store additional data that isn't part of the standard header, such as spatial reference
//! information.

mod raw;
mod vlr;

pub use vlr::raw::RawVlr;
pub use vlr::vlr::Vlr;
