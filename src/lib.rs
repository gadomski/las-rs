//! Read and write [ASPRS LAS](https://www.asprs.org/committee-general/laser-las-file-format-exchange-activities.html)
//! point cloud data.
//!
//! # Reading
//!
//! Create a [Reader] from a [Path](std::path::Path):
//!
//! ```
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! ```
//!
//! Or anything that implements [Read](std::io::Read):
//!
//! ```
//! use std::io::BufReader;
//! use std::fs::File;
//! use las::Reader;
//! let read = BufReader::new(File::open("tests/data/autzen.las").unwrap());
//! let reader = Reader::new(read).unwrap();
//! ```
//!
//! ## Prefer [BufRead](std::io::BufRead)
//!
//! Your performance will be better if your [Read](std::io::Read) is actually a
//! [BufRead](std::io::BufRead). [Reader::from_path] takes care of this for you,
//! but [Reader::new] doesn't.
//!
//! ## Read points
//!
//! Read points one-by-one with [Reader::read]:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let point = reader.read().unwrap().unwrap();
//! ```
//!
//! Or iterate over all points with [Reader::points]:
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
//! Create a [Writer] from a [Write](std::io::Write) and a [Header]:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Header};
//! let write = Cursor::new(Vec::new());
//! let header = Header::default();
//! let writer = Writer::new(write, header).unwrap();
//! ```
//!
//! You can also write out to a path (automatically buffered with [BufWriter](std::io::BufWriter)):
//!
//! ```
//! use las::Writer;
//! let writer = Writer::from_path("/dev/null", Default::default());
//! ```
//!
//! Use a [Builder] to customize the las data:
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
//! ## Prefer [BufWriter](std::io::BufWriter)
//!
//! Just like the [Reader], your performance will improve greatly if you use a
//! [BufWriter](std::io::BufWriter) instead of just a [Write](std::io::Write).
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
//!
//! # Compression
//!
//! The [laz](https://laszip.org/) compression format is the de-facto standard for compression las data.
//! To enable laz support, enable the `laz` or `laz-parallel` feature:
//!
//! ```toml
//! [dependencies]
//! las = { version = "0.9", features = ["laz"] }  # or laz-parallel
//! ```
//!
//! Then, you can compress the data when writing:
//!
//! ```
//! use std::io::Cursor;
//! use las::{Writer, Builder};
//! use las::point::Format;
//!
//! let mut builder = Builder::from((1, 4));
//! builder.point_format = Format::new(2).unwrap();
//! builder.point_format.is_compressed = true;
//! let header = builder.into_header().unwrap();
//! let write = Cursor::new(Vec::new());
//! let result =  Writer::new(write, header);
//! if cfg!(feature = "laz") {
//!     assert!(result.is_ok());
//! } else {
//!     assert!(result.is_err());
//! }
//! ```
//!
//! [Writer::from_path] will use the extension of the output file to determine
//! wether the data should be compressed or not:
//!
//! - `.laz`: compressed
//! - `.las`: not compressed

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

#[cfg(feature = "laz")]
pub mod copc;
#[cfg(feature = "laz")]
pub mod laz;

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

#[cfg(feature = "laz")]
pub use crate::copc::CopcEntryReader;
pub use crate::{
    bounds::Bounds,
    color::Color,
    error::Error,
    feature::Feature,
    gps_time_type::GpsTimeType,
    header::{Builder, Header},
    point::Point,
    reader::{Reader, ReaderOptions},
    transform::Transform,
    vector::Vector,
    version::Version,
    vlr::Vlr,
    writer::{Writer, WriterOptions},
};
#[cfg(feature = "laz")]
pub use reader::LazParallelism;
#[allow(deprecated)]
pub use {reader::Read, writer::Write};

/// Crate-specific result type.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
use criterion as _;
