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
//! use las::{Read, Reader};
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let point = reader.read().unwrap().unwrap();
//! ```
//!
//! Or iterate over all points with `Reader::points`:
//!
//! ```
//! use las::{Read, Reader};
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
//! use las::{Write, Writer, Point};
//! let mut writer = Writer::default();
//! let point = Point { x: 1., y: 2., z: 3., ..Default::default() };
//! writer.write(point).unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(
    elided_lifetimes_in_paths,
    explicit_outlives_requirements,
    keyword_idents,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_abi,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    noop_method_call,
    pointer_structural_match,
    rust_2021_incompatible_closure_captures,
    rust_2021_incompatible_or_patterns,
    rust_2021_prefixes_incompatible_syntax,
    rust_2021_prelude_collisions,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    warnings
)]
#![recursion_limit = "128"]

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

pub use crate::bounds::Bounds;
pub use crate::color::Color;
pub use crate::error::Error;
pub use crate::feature::Feature;
pub use crate::gps_time_type::GpsTimeType;
pub use crate::header::{Builder, Header};
pub use crate::point::Point;
pub use crate::reader::{Read, Reader};
pub use crate::transform::Transform;
pub use crate::vector::Vector;
pub use crate::version::Version;
pub use crate::vlr::Vlr;
pub use crate::writer::{Write, Writer};

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
use criterion as _;
