//! Read las points.
//!
//! If you're reading any significant number of points, you'll want to make sure you're using a
//! `BufRead` instead of just a `Read`:
//!
//! ```
//! use std::fs::File;
//! use std::io::BufReader;
//! use las::Reader;
//!
//! let read = BufReader::new(File::open("tests/data/autzen.las").unwrap());
//! let reader = Reader::new(read).unwrap();
//! ```
//!
//! `Reader::from_path` does this for you:
//!
//! ```
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! ```
//!
//! Read points into a [`PointData`] — either the whole file, a chunk of
//! `n`, or into a reusable buffer. From a [`PointData`] you iterate
//! decoded [`Point`](crate::Point) values with [`PointData::points`] or
//! sweep single columns (x, y, z, intensity, …) directly:
//!
//! ```
//! use las::Reader;
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let pd = reader.read_all().unwrap();
//! let all_points = pd.points().map(|r| r.unwrap()).collect::<Vec<_>>();
//! ```
//!
//! # Compression
//!
//! [lazip](https://laszip.org/) is supported by enabling the `laz` feature in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! las = { version = "*", features = ["laz"] }
//! ```
//!
//! Then:
//!
//! ```
//! # #[cfg(feature = "laz")]
//! # {
//! use las::Reader;
//! let reader = Reader::from_path("tests/data/autzen.laz").unwrap();
//! # }
//! ```
//!

mod las;
#[cfg(feature = "laz")]
mod laz;

use crate::{Error, Header, PointData, Result};
use std::{
    fs::File,
    io::{BufReader, Seek},
    path::Path,
};

/// Backend-specific byte-slab filler.
///
/// LAS and LAZ backends know how to materialize up to `n` raw point
/// records as raw on-disk bytes into a caller-provided `Vec<u8>`. The
/// trait is bytes-only so [`PointData`] doesn't leak through backend
/// boundaries — the reader's [`Reader::fill_points`] wraps the byte-slab
/// fill around a `PointData`'s storage.
trait ReadPoints {
    /// Resizes `out` to `n * record_len` bytes and fills it with up to `n`
    /// raw point records. Returns the number of points actually read
    /// (never more than `n`, limited by remaining points in the file).
    fn fill_into_bytes(
        &mut self,
        n: u64,
        out: &mut Vec<u8>,
        record_len: usize,
    ) -> Result<u64>;
    fn seek(&mut self, index: u64) -> Result<()>;
    fn header(&self) -> &Header;
}

/// Choice of laz parallelism.
#[cfg(feature = "laz")]
#[derive(Debug, Clone, Copy)]
pub enum LazParallelism {
    #[cfg(feature = "laz-parallel")]
    /// Use parallel laz decompression / compression
    Yes,
    /// Do not use parallel laz decompression / compression
    No,
}

/// Options for Reader.
///
/// Currently, the only option is the selection of LAZ parallelism via [LazParallelism].
/// This option requires the `laz` feature to be enabled (and to use parallelism, the `laz-parallel`
/// feature must also be enabled)
/// By default, if the `laz-parallel` feature is enabled, parallelism will be the default choice
#[derive(Debug, Clone, Copy)]
pub struct ReaderOptions {
    #[cfg(feature = "laz")]
    laz_parallelism: LazParallelism,
}

impl ReaderOptions {
    /// Change the laz parallelism option
    #[cfg(feature = "laz")]
    pub fn with_laz_parallelism(mut self, laz_parallelism: LazParallelism) -> Self {
        self.laz_parallelism = laz_parallelism;
        self
    }
}

impl Default for ReaderOptions {
    fn default() -> Self {
        #[cfg(feature = "laz-parallel")]
        {
            Self {
                laz_parallelism: LazParallelism::Yes,
            }
        }
        #[cfg(all(feature = "laz", not(feature = "laz-parallel")))]
        {
            Self {
                laz_parallelism: LazParallelism::No,
            }
        }

        #[cfg(not(feature = "laz"))]
        {
            Self {}
        }
    }
}

/// Reads LAS data into [`PointData`].
#[allow(missing_debug_implementations)]
pub struct Reader {
    point_reader: Box<dyn ReadPoints>,
}

impl Reader {
    /// Creates a new reader with default options.
    ///
    /// This does *not* wrap the `Read` in a `BufRead`, so if you're concerned
    /// about performance you should do that wrapping yourself (or use
    /// `from_path`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::BufReader;
    /// use std::fs::File;
    /// # use las::Reader;
    /// let file = File::open("tests/data/autzen.las").unwrap();
    /// let reader = Reader::new(BufReader::new(file)).unwrap();
    /// ```
    pub fn new<R: std::io::Read + Seek + Send + Sync + 'static>(read: R) -> Result<Reader> {
        Self::with_options(read, ReaderOptions::default())
    }

    /// Creates a new reader with custom options.
    ///
    /// This does *not* wrap the `Read` in a `BufRead`, so if you're concerned
    /// about performance you should do that wrapping yourself (or use
    /// `from_path`).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::BufReader;
    /// use std::fs::File;
    /// # use las::{Reader, ReaderOptions};
    /// let file = File::open("tests/data/autzen.las").unwrap();
    /// let reader = Reader::with_options(BufReader::new(file), ReaderOptions::default()).unwrap();
    /// ```
    pub fn with_options<R: std::io::Read + Seek + Send + Sync + 'static>(
        mut read: R,
        options: ReaderOptions,
    ) -> Result<Reader> {
        let header = Header::new(&mut read)?;
        if header.point_format().is_compressed {
            #[cfg(feature = "laz")]
            {
                let point_reader: Box<dyn ReadPoints> = match options.laz_parallelism {
                    #[cfg(feature = "laz-parallel")]
                    LazParallelism::Yes => {
                        laz::PointReader::new_parallel(read, header).map(Box::new)?
                    }
                    LazParallelism::No => laz::PointReader::new(read, header).map(Box::new)?,
                };

                Ok(Reader { point_reader })
            }
            #[cfg(not(feature = "laz"))]
            {
                Err(Error::LaszipNotEnabled)
            }
        } else {
            // Silence unused variable warning as the only option is related to laz
            let _ = options;
            Ok(Reader {
                point_reader: Box::new(las::PointReader::new(read, header)?),
            })
        }
    }

    /// Creates a new reader from a path.
    ///
    /// The underlying `File` is wrapped in a `BufReader` for performance reasons.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Reader> {
        File::open(path)
            .map_err(Error::from)
            .and_then(|file| Reader::new(BufReader::new(file)))
    }

    /// Returns a reference to this reader's header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let header = reader.header();
    /// ```
    pub fn header(&self) -> &Header {
        self.point_reader.header()
    }

    /// Reads up to `n` points into a fresh [`PointData`].
    ///
    /// The returned [`PointData`] contains the next `n` points (or fewer
    /// if the file is exhausted), with this reader's format and coordinate
    /// transforms. Use [`PointData::points`] to walk it as owned [`Point`]
    /// values, or the column accessors ([`PointData::x`],
    /// [`PointData::intensity`], …) for bulk field extraction.
    ///
    /// For buffer reuse across many calls, prefer [`Reader::fill_points`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let pd = reader.read_points(10).unwrap();
    /// assert_eq!(pd.len(), 10);
    /// ```
    pub fn read_points(&mut self, n: u64) -> Result<PointData> {
        let mut pd = PointData::new(
            *self.point_reader.header().point_format(),
            *self.point_reader.header().transforms(),
        );
        let _ = self.fill_points(n, &mut pd)?;
        Ok(pd)
    }

    /// Reads every remaining point into a fresh [`PointData`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let pd = reader.read_all().unwrap();
    /// assert_eq!(pd.len(), reader.header().number_of_points() as usize);
    /// ```
    pub fn read_all(&mut self) -> Result<PointData> {
        let remaining = self.point_reader.header().number_of_points();
        self.read_points(remaining)
    }

    /// Fills `target` with up to `n` points, replacing its contents.
    ///
    /// Reuses `target`'s underlying byte buffer — use this in loops that
    /// process a file in batches to avoid per-iteration allocations. If
    /// `target`'s format doesn't match this reader's, `target` is
    /// reinitialized to the reader's format and transforms before filling.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::{Reader, PointData};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let mut pd = PointData::new(
    ///     *reader.header().point_format(),
    ///     *reader.header().transforms(),
    /// );
    /// let n = reader.fill_points(10, &mut pd).unwrap();
    /// assert_eq!(n, 10);
    /// ```
    pub fn fill_points(&mut self, n: u64, target: &mut PointData) -> Result<u64> {
        let format = *self.point_reader.header().point_format();
        let transforms = *self.point_reader.header().transforms();
        if target.format() != &format {
            *target = PointData::new(format, transforms);
        }
        let record_len = target.record_len();
        let bytes = target.take_bytes_mut();
        self.point_reader.fill_into_bytes(n, bytes, record_len)
    }

    /// Seeks to the given point number, zero-indexed.
    ///
    /// Note that seeking on compressed (LAZ) data can be expensive as the
    /// reader will have to seek to the closest chunk start and decompress
    /// all points up until the point seeked to.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Reader;
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// reader.seek(1).unwrap(); // <- seeks to the second point
    /// let pd = reader.read_points(1).unwrap();
    /// ```
    pub fn seek(&mut self, position: u64) -> Result<()> {
        self.point_reader.seek(position)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Writer};

    #[test]
    fn seek() {
        let mut writer = Writer::default();
        writer.write_point(Default::default()).unwrap();
        let point = Point {
            x: 1.,
            y: 2.,
            z: 3.,
            ..Default::default()
        };
        writer.write_point(point.clone()).unwrap();
        let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
        reader.seek(1).unwrap();
        let pd = reader.read_points(1).unwrap();
        assert_eq!(pd.len(), 1);
        let got = pd.points().next().unwrap().unwrap();
        assert_eq!(point, got);
        let rest = reader.read_points(1).unwrap();
        assert!(rest.is_empty());
    }
}
