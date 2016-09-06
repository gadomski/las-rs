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
//!
//! // Points from `Reader::read` are provided as a `Result<Option<Point>>`
//! let point = reader.read().unwrap().unwrap();
//!
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
//! writer.write(&point).unwrap();
//! ```
//!
//! In order to configure the `Writer`, e.g. to set the LAS version or point format, use a
//! `Builder`:
//!
//! ```
//! # use std::io::Cursor;
//! use las::Builder;
//! let mut builder = Builder::new();
//! builder.point_format = 1.into();
//! builder.version = (1, 2).into();
//! let writer = builder.writer(Cursor::new(Vec::new())).unwrap();
//! ```
//!
//! Convenience methods are provided for writing LAS data to a file:
//!
//! ```
//! # use las::{Writer, Builder};
//! // Uses the default writer:
//! let writer = Writer::from_path("/dev/null").unwrap();
//!
//! // Allows configuration before open:
//! let writer = Builder::new().writer_from_path("/dev/null").unwrap();
//! ```
//!
//! A `Writer` implements `Drop`, which it uses to re-write the header with the point count and
//! other metadata when the `Writer` goes out of scope. If this header re-write fails, an error
//! will be printed to the logs but the thread will not panic. If is unacceptable, you can close
//! the `Writer` yourself and prevent any re-writing on drop:
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
#[macro_use]
extern crate log;

pub mod global_encoding;
pub mod point;
pub mod utils;

mod builder;
mod error;
mod header;
mod reader;
mod version;
mod vlr;
mod writer;

pub use builder::Builder;
pub use error::Error;
pub use header::Header;
pub use point::Point;
pub use reader::Reader;
pub use writer::Writer;
pub use version::Version;
pub use vlr::Vlr;

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;
