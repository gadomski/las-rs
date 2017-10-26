//! Natively read and write [ASPRS LAS]
//! (https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html) data.
//!
//! # Reading
//!
//! `Reader`s can be created from paths:
//!
//! ```
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! ```
//!
//! Or anything that implements `Read`:
//!
//! ```
//! use std::io::BufReader;
//! use std::fs::File;
//! use las::Reader;
//! let read = BufReader::new(File::open("tests/data/autzen.las").unwrap());
//! let reader = Reader::new(read).unwrap();
//! ```
//!
//! Your performance will be better if your `Read` is actually a `BufRead`. `Reader::from_path`
//! takes care of this for you, but `Reader::new` doesn't.
//!
//! Read points one-by-one with `read`:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let point = reader.read().unwrap().unwrap();
//! ```
//!
//! # Writing
//!
//! Create a `Writer` from something that implements `Write` and a `Header`:
//!
//! ```
//! use std::io::Cursor;
//! use las::Writer;
//! let writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
//! ```
//!
//! Use the `Header` to customize the output data formats:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Header};
//! use las::point::Format;
//! let header = Header {
//!     version: (1, 3).into(),
//!     point_format: Format::new(2).unwrap(),
//!     ..Default::default()
//! };
//! let writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
//! ```
//!
//! You can also write out to a path (automatically buffered):
//!
//! ```
//! use las::Writer;
//! let writer = Writer::from_path("/dev/null", Default::default());
//! ```
//!
//! Write points one at a time:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Point};
//! let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
//! let point = Point { x: 1., y: 2., z: 3., ..Default::default() };
//! writer.write(&point).unwrap();
//! ```

#![deny(missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]
#![recursion_limit="128"]

extern crate byteorder;
extern crate chrono;
extern crate num;
#[macro_use]
extern crate quick_error;

pub mod point;
pub mod raw;
pub mod reader;

mod bounds;
mod color;
mod error;
mod gps_time_type;
mod header;
mod transform;
mod utils;
mod vector;
mod version;
mod vlr;
mod writer;

pub use bounds::Bounds;
pub use color::Color;
pub use error::Error;
pub use gps_time_type::GpsTimeType;
pub use header::Header;
pub use point::Point;
pub use reader::Reader;
pub use transform::Transform;
pub use vector::Vector;
pub use version::Version;
pub use vlr::Vlr;
pub use writer::Writer;

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;
