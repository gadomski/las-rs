//! Read and write [ASPRS LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html)
//! point cloud data.
//!
//! # Reading
//!
//! Create a `Reader` from a `Path`:
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
//! ## Prefer `BufRead`
//!
//! Your performance will be better if your `Read` is actually a `BufRead`. `Reader::from_path`
//! takes care of this for you, but `Reader::new` doesn't.
//!
//! ## Read points
//!
//! Read points one-by-one with `Reader::read`:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let point = reader.read().unwrap().unwrap();
//! ```
//!
//! Or iterate over all points with `Reader::points`:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! for wrapped_point in reader.points() {
//!     let point = wrapped_point.unwrap();
//!     println!("Point coordinates: ({}, {}, {})", point.x, point.y, point.z);
//!     if let Some(color) = point.color {
//!         println!("Point color: red={}, green={}, blue={}",
//!             color.red,
//!             color.green,
//!             color.blue,
//!         );
//!     }
//! }
//! ```
//!
//! # Writing
//!
//! Create a `Writer` from a `Write` and a `Header`:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Header};
//! let write = Cursor::new(Vec::new());
//! let header = Header::default();
//! let writer = Writer::new(write, header).unwrap();
//! ```
//!
//! Use the `Header` to customize the output data formats:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Header};
//! use las::point::Format;
//! let write = Cursor::new(Vec::new());
//! let mut header = Header::default();
//! header.version = (1, 3).into();
//! header.point_format = Format::new(2).unwrap();
//! let writer = Writer::new(write, header).unwrap();
//! ```
//!
//! You can also write out to a path (automatically buffered with `BufWriter`):
//!
//! ```
//! use las::Writer;
//! let writer = Writer::from_path("/dev/null", Default::default());
//! ```
//!
//! ## Prefer `BufWrite`
//!
//! Just like the `Reader`, your performance will improve greatly if you use a `BufWrite` instead
//! of just a `Write`.
//!
//! ## Write points
//!
//! Write points one at a time:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Point};
//! let mut writer = Writer::new(Cursor::new(Vec::new()), Default::default()).unwrap();
//! let point = Point { x: 1., y: 2., z: 3., ..Default::default() };
//! writer.write(point).unwrap();
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
extern crate uuid;

pub mod feature;
pub mod header;
pub mod point;
pub mod raw;
pub mod reader;
pub mod vlr;
pub mod writer;

mod bounds;
mod color;
mod error;
mod gps_time_type;
mod transform;
mod utils;
mod vector;
mod version;

pub use bounds::Bounds;
pub use color::Color;
pub use error::Error;
pub use feature::Feature;
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
