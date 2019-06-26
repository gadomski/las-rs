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
//! You can also write out to a path (automatically buffered with `BufWriter`):
//!
//! ```
//! use las::Writer;
//! let writer = Writer::from_path("/dev/null", Default::default());
//! ```
//!
//! Use a `Builder` to customize the las data:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Builder};
//! use las::point::Format;
//!
//! let mut builder = Builder::from((1, 4));
//! builder.point_format = Format::new(2).unwrap();
//! let header = builder.into_header().unwrap();
//!
//! let write = Cursor::new(Vec::new());
//! let writer = Writer::new(write, header).unwrap();
//! ```
//!
//! If compiled with laz you can compress the data written
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Builder};
//! use las::point::Format;
//!
//! let mut builder = Builder::from((1, 4));
//! builder.point_format = Format::new(2).unwrap();
//! // asking to compress data
//! builder.point_format.is_compressed = true;
//! let header = builder.into_header().unwrap();
//!
//! let write = Cursor::new(Vec::new());
//! let is_compiled_with_laz = cfg!(feature = "laz");
//!
//!
//! let result =  Writer::new(write, header);
//! if is_compiled_with_laz {
//!     assert_eq!(result.is_ok(), true);
//! } else {
//!    assert_eq!(result.is_err(), true);
//! }
//!
//! ```
//!
//! The [from_path](writer/struct.Writer.html#method.from_path) will use the extension of the output
//! file to determine wether the data should be compressed or not
//! 'laz' => compressed
//! 'las' => not compressed
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
//! let mut writer = Writer::default();
//! let point = Point { x: 1., y: 2., z: 3., ..Default::default() };
//! writer.write(point).unwrap();
//! ```

#![deny(missing_docs,
        missing_debug_implementations,
        missing_copy_implementations,
        trivial_casts,
        trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces,
        unused_qualifications)]

#![recursion_limit="128"]

extern crate byteorder;
extern crate chrono;
extern crate num;
#[macro_use]
extern crate quick_error;
extern crate uuid;

#[cfg(feature = "laz")]
extern crate laz;

#[cfg(feature = "laz")]
mod compression;

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
pub use header::{Header, Builder};
pub use point::Point;
pub use reader::Reader;
pub use transform::Transform;
pub use vector::Vector;
pub use version::Version;
pub use vlr::Vlr;
pub use writer::Writer;

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;
