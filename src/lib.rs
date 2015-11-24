//! Reads and writes point cloud data stored in the ASPRS las file format.

#![deny(missing_copy_implementations, missing_debug_implementations, missing_docs, trivial_casts,
        trivial_numeric_casts, unsafe_code, unstable_features, unused_extern_crates,
        unused_import_braces, unused_qualifications)]

extern crate byteorder;
extern crate time;

pub mod header;
pub mod error;
pub mod file;
pub mod point;
mod io;
mod scale;
pub mod stream;
pub mod vlr;
pub mod writer;

pub use error::Error;
pub use file::File;
pub use point::Point;
pub use stream::Stream;
pub use writer::Writer;

use std::result;

/// Crate-specific resuls.
pub type Result<T> = result::Result<T, Error>;
