//! Native library for reading and writing the [ASPRS
//! LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html)
//! data exchange format.
//!
//! The LAS data exchange format is designed for transmitting and storing
//! [LiDAR](https://en.wikipedia.org/wiki/Lidar) data.
//!
//! # Reading points
//!
//! Use a `Reader` to read one or more points:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("data/1.0_0.las").unwrap();
//! // Points from `Reader::read` are provided as a `Result<Option<Point>>`
//! let point = reader.read().unwrap().unwrap();
//! // Use `.iter_mut` to iterate over points, provided as `Result<Point>`.
//! for point in reader.iter_mut() {
//!     let point = point.unwrap();
//!     let x = point.x;
//!     // etc.
//! }
//! ```
//!
//! # Writing points
//!
//! A `Writer` writes points to a `Read`. If you're comfortable with reasonable default settings,
//! use a `Writer` directly:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Point, Writer};
//! let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
//! let mut point: Point = Default::default();
//! point.x = 1.;
//! // etc.
//! writer.write(point).unwrap();
//! ```
//!
//! In order to configure the `Writer`, i.e. by setting the LAS version or point format, use a
//! `Builder`:
//!
//! ```
//! # use std::io::Cursor;
//! use las::{Builder, point, Version};
//! let writer = Builder::new()
//!     .point_format(point::Format::from(1))
//!     .version(Version::new(1, 2))
//!     .writer(Cursor::new(Vec::new())).unwrap();
//! ```
//!
//! There are no file-based operations on a `Writer`. To write data to a file, you have to create
//! the file yourself:
//!
//! ```
//! # use las::Writer;
//! use std::fs::File;
//! let writer = Writer::default(File::create("/dev/null").unwrap()).unwrap();
//! ```
//!
//! A `Writer` implements `Drop`, which it uses to re-write the header with the point count and
//! other metadata when the `Writer` goes out of scope. If this header re-write fails, a panic will
//! result. If is unacceptable, you can manually close to the `Writer` and prevent any re-writing:
//!
//! ```
//! # use std::io::Cursor;
//! # use las::Writer;
//! {
//!     let mut writer = Writer::default(Cursor::new(Vec::new())).unwrap();
//!     writer.close().unwrap();
//! } // `close` is not called
//! ```

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]

extern crate byteorder;
extern crate chrono;

pub mod global_encoding;
pub mod point;
pub mod utils;

mod error;
mod header;
mod reader;
mod version;
mod vlr;
mod writer;

pub use error::Error;
pub use header::Header;
pub use point::Point;
pub use reader::Reader;
pub use writer::{Builder, Writer};
pub use version::Version;
pub use vlr::Vlr;

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;
