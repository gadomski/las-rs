//! A byte-slab point cloud, for high-throughput bulk reads.
//!
//! [Points] holds a single contiguous `Vec<u8>` of decompressed LAS point
//! records in their on-disk layout. It is the "bytes-out" counterpart to the
//! [Point](crate::Point) iteration API on [Reader](crate::Reader), and is
//! intended for the throughput case: decoding millions of points at a time,
//! or computing column-oriented statistics (bounds, means, histograms)
//! without paying the per-point [Point](crate::Point) materialization cost.
//!
//! The [Point](crate::Point) / [Reader::read_points_into](crate::Reader::read_points_into)
//! API remains the best choice for scripts and one-off operations; `Points`
//! is the byte-slab alternative.
//!
//! # Example
//!
//! ```
//! use las::{Reader, Points};
//!
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let points = Points::read_all(&mut reader).unwrap();
//!
//! // Row-oriented: iterate points as owned `Point` values.
//! for p in points.iter().take(3) {
//!     let p = p.unwrap();
//!     println!("{:.3} {:.3} {:.3} intensity={}", p.x, p.y, p.z, p.intensity);
//! }
//!
//! // Column-oriented: one pass over the x column, no `Point` materialization.
//! let min_x = points.x().fold(f64::INFINITY, f64::min);
//! let max_x = points.x().fold(f64::NEG_INFINITY, f64::max);
//! assert!(min_x <= max_x);
//! ```

use crate::{point::Format, raw, raw::point::Layout, Point, Reader, Result, Transform, Vector};
use std::io::Cursor;

/// A set of decompressed LAS point records held as one contiguous byte slab.
///
/// `Points` mirrors the on-disk layout for a specific point format. Construct
/// one with [Points::from_reader] or [Points::read_all] (reader-backed), with
/// [Points::from_raw_bytes] to wrap an existing `Vec<u8>`, or start from
/// [Points::new] and fill via [Points::fill_from] for buffer reuse or
/// [Points::resize_for] for custom decompressors. The whole row is
/// materializable as [Point] via [Points::iter], and columns can be iterated
/// field-by-field via [Points::x], [Points::intensity], and so on without
/// paying the full per-point decode cost.
///
/// # Example
///
/// ```
/// use las::{Reader, Points};
///
/// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
/// let points = Points::from_reader(&mut reader, 10).unwrap();
/// assert_eq!(points.len(), 10);
/// ```
#[derive(Clone, Debug)]
pub struct Points {
    bytes: Vec<u8>,
    format: Format,
    transforms: Vector<Transform>,
    layout: Layout,
}

impl Points {
    /// Creates an empty `Points` for the given format and coordinate
    /// transforms.
    ///
    /// The format and transforms are cached for per-point accessor dispatch,
    /// and must match the file these records will be filled from. In typical
    /// usage, pass `*reader.header().point_format()` and
    /// `*reader.header().transforms()`.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Reader, Points};
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = Points::new(
    ///     *reader.header().point_format(),
    ///     *reader.header().transforms(),
    /// );
    /// assert!(points.is_empty());
    /// ```
    pub fn new(format: Format, transforms: Vector<Transform>) -> Self {
        let layout = Layout::for_format(&format);
        Points {
            bytes: Vec::new(),
            format,
            transforms,
            layout,
        }
    }

    /// Creates a `Points` by wrapping an existing byte buffer.
    ///
    /// The byte buffer must be a sequence of tightly-packed point records for
    /// the given format — i.e. its length must be a multiple of the format's
    /// record length. This is useful for callers that already have a
    /// decompressed `Vec<u8>` in hand (custom decompressor, memory-mapped
    /// region, received over the wire) and want to hand it over without an
    /// extra allocation or copy.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCloudByteLength`](crate::Error::InvalidCloudByteLength)
    /// if `bytes.len()` is not a multiple of the format's record length.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Points, point::Format, Transform, Vector};
    ///
    /// let format = Format::new(0).unwrap();
    /// let transforms: Vector<Transform> = Default::default();
    /// let record_len = format.len() as usize;
    /// let bytes = vec![0u8; record_len * 5];
    /// let points = Points::from_raw_bytes(format, transforms, bytes).unwrap();
    /// assert_eq!(points.len(), 5);
    /// ```
    pub fn from_raw_bytes(
        format: Format,
        transforms: Vector<Transform>,
        bytes: Vec<u8>,
    ) -> Result<Self> {
        let layout = Layout::for_format(&format);
        if layout.record_len == 0 || !bytes.len().is_multiple_of(layout.record_len) {
            return Err(crate::Error::InvalidCloudByteLength {
                len: bytes.len(),
                record_len: layout.record_len,
            });
        }
        Ok(Points {
            bytes,
            format,
            transforms,
            layout,
        })
    }

    /// Reads up to `n` points from `reader` into a fresh `Points`.
    ///
    /// The returned `Points` uses the reader's point format and coordinate
    /// transforms. If the reader has fewer than `n` points remaining, the
    /// returned `Points` holds only what was available.
    ///
    /// This is the high-throughput entry point: it skips per-point [Point]
    /// materialization and fills the byte slab directly from the LAZ
    /// decompressor (or the raw reader, for uncompressed files). For column
    /// sweeps or a single-column statistic (min/max/mean), prefer this over
    /// [Reader::read_points_into].
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Reader, Points};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let points = Points::from_reader(&mut reader, 10).unwrap();
    /// assert_eq!(points.len(), 10);
    /// ```
    pub fn from_reader(reader: &mut Reader, n: u64) -> Result<Self> {
        let mut points = Points::new(
            *reader.header().point_format(),
            *reader.header().transforms(),
        );
        let _ = reader.fill_points_slab(n, &mut points)?;
        Ok(points)
    }

    /// Reads every remaining point from `reader` into a fresh `Points`.
    ///
    /// Convenience for the common case of "give me the whole file as a slab".
    /// For very large files where you want to process points in batches,
    /// prefer [Points::fill_from] on a reusable `Points`.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Reader, Points};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let total = reader.header().number_of_points() as usize;
    /// let points = Points::read_all(&mut reader).unwrap();
    /// assert_eq!(points.len(), total);
    /// ```
    pub fn read_all(reader: &mut Reader) -> Result<Self> {
        let remaining = reader.header().number_of_points();
        Points::from_reader(reader, remaining)
    }

    /// Fills `self` with up to `n` points from `reader`, replacing any
    /// existing contents. Returns the number of points decoded.
    ///
    /// Reuses `self`'s underlying byte buffer, so this is the right choice
    /// for loops that process a file in batches.
    ///
    /// If `self`'s point format doesn't match the reader's, `self` is
    /// reinitialized to the reader's format and transforms before filling.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Reader, Points};
    /// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let mut points = Points::new(
    ///     *reader.header().point_format(),
    ///     *reader.header().transforms(),
    /// );
    /// let n = points.fill_from(&mut reader, 10).unwrap();
    /// assert_eq!(n, 10);
    /// assert_eq!(points.len(), 10);
    /// ```
    pub fn fill_from(&mut self, reader: &mut Reader, n: u64) -> Result<u64> {
        reader.fill_points_slab(n, self)
    }

    /// Returns the number of points currently held.
    pub fn len(&self) -> usize {
        self.bytes.len().checked_div(self.layout.record_len).unwrap_or(0)
    }

    /// Returns true if this `Points` contains no points.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the point format these records were built for.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Returns the coordinate transforms these records were built with.
    pub fn transforms(&self) -> &Vector<Transform> {
        &self.transforms
    }

    /// Returns the underlying byte buffer.
    ///
    /// Its length is `self.len() * self.format().len() as usize`. Callers that
    /// want to parse fields themselves or hand the slab to another system can
    /// use this as an escape hatch.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the record length in bytes for this format.
    pub fn record_len(&self) -> usize {
        self.layout.record_len
    }

    /// Iterates points row-by-row as owned [Point] values.
    ///
    /// Each point is materialized through the same [`raw::Point::read_from`] +
    /// [`Point::new`] pipeline that
    /// [`Reader::read_points_into`](crate::Reader::read_points_into) uses, so
    /// per-point cost is identical; prefer the column accessors
    /// ([`Points::x`], [`Points::intensity`], …) when you only need a subset
    /// of fields.
    pub fn iter(&self) -> PointIter<'_> {
        PointIter {
            cursor: Cursor::new(&self.bytes),
            format: &self.format,
            transforms: &self.transforms,
            remaining: self.len(),
        }
    }

    /// Resizes the underlying byte buffer to hold exactly `n` points and
    /// returns a mutable view of it.
    ///
    /// This is the primary entry point for callers that drive a decompressor
    /// directly (e.g. against COPC chunks that bypass [Reader](crate::Reader)).
    /// Call `resize_for`, decompress into the returned slice, and then use the
    /// column accessors or [Points::iter] as usual.
    pub fn resize_for(&mut self, n: usize) -> &mut [u8] {
        let new_len = n.checked_mul(self.layout.record_len).expect("overflow");
        self.bytes.resize(new_len, 0u8);
        &mut self.bytes
    }

    /// Raw scaled x values (little-endian i32 loads from the x column).
    pub fn x_raw(&self) -> impl Iterator<Item = i32> + '_ {
        self.i32_column(0)
    }

    /// Raw scaled y values.
    pub fn y_raw(&self) -> impl Iterator<Item = i32> + '_ {
        self.i32_column(4)
    }

    /// Raw scaled z values.
    pub fn z_raw(&self) -> impl Iterator<Item = i32> + '_ {
        self.i32_column(8)
    }

    /// World x values, with scale and offset applied.
    pub fn x(&self) -> impl Iterator<Item = f64> + '_ {
        let t = self.transforms.x;
        self.x_raw().map(move |n| t.direct(n))
    }

    /// World y values.
    pub fn y(&self) -> impl Iterator<Item = f64> + '_ {
        let t = self.transforms.y;
        self.y_raw().map(move |n| t.direct(n))
    }

    /// World z values.
    pub fn z(&self) -> impl Iterator<Item = f64> + '_ {
        let t = self.transforms.z;
        self.z_raw().map(move |n| t.direct(n))
    }

    /// Intensity column.
    pub fn intensity(&self) -> impl Iterator<Item = u16> + '_ {
        self.u16_column(12)
    }

    /// Classification byte column. For legacy formats this is the low 5 bits
    /// of the second flags byte; for extended formats it is the third flags
    /// byte directly.
    pub fn classification(&self) -> impl Iterator<Item = u8> + '_ {
        let is_extended = self.format.is_extended;
        self.records().map(move |rec| {
            if is_extended {
                rec[16]
            } else {
                rec[15] & 0b0001_1111
            }
        })
    }

    /// Return number column.
    pub fn return_number(&self) -> impl Iterator<Item = u8> + '_ {
        let is_extended = self.format.is_extended;
        self.records().map(move |rec| {
            if is_extended {
                rec[14] & 15
            } else {
                rec[14] & 7
            }
        })
    }

    /// Number-of-returns column.
    pub fn number_of_returns(&self) -> impl Iterator<Item = u8> + '_ {
        let is_extended = self.format.is_extended;
        self.records().map(move |rec| {
            if is_extended {
                (rec[14] >> 4) & 15
            } else {
                (rec[14] >> 3) & 7
            }
        })
    }

    /// Scan angle column, in degrees.
    ///
    /// Legacy formats store scan angle as an `i8` rank in `[-90, 90]` degrees.
    /// Extended formats store it as an `i16` in units of `0.006` degrees. Both
    /// are normalized to `f32` degrees here.
    pub fn scan_angle_degrees(&self) -> impl Iterator<Item = f32> + '_ {
        let is_extended = self.format.is_extended;
        let sa_off = self.layout.scan_angle;
        self.records().map(move |rec| {
            if is_extended {
                let raw = i16::from_le_bytes([rec[sa_off], rec[sa_off + 1]]);
                f32::from(raw) * 0.006
            } else {
                f32::from(rec[sa_off] as i8)
            }
        })
    }

    /// User data byte column.
    pub fn user_data(&self) -> impl Iterator<Item = u8> + '_ {
        let ud = self.layout.user_data;
        self.records().map(move |rec| rec[ud])
    }

    /// Point source ID column.
    pub fn point_source_id(&self) -> impl Iterator<Item = u16> + '_ {
        let ps = self.layout.point_source_id;
        self.records()
            .map(move |rec| u16::from_le_bytes([rec[ps], rec[ps + 1]]))
    }

    /// GPS time column, or `None` if the format has no gps_time field.
    pub fn gps_time(&self) -> Option<impl Iterator<Item = f64> + '_> {
        let g = self.layout.gps_time?;
        Some(self.records().map(move |rec| {
            f64::from_le_bytes([
                rec[g],
                rec[g + 1],
                rec[g + 2],
                rec[g + 3],
                rec[g + 4],
                rec[g + 5],
                rec[g + 6],
                rec[g + 7],
            ])
        }))
    }

    /// RGB column, or `None` if the format has no color.
    pub fn rgb(&self) -> Option<impl Iterator<Item = (u16, u16, u16)> + '_> {
        let c = self.layout.rgb?;
        Some(self.records().map(move |rec| {
            (
                u16::from_le_bytes([rec[c], rec[c + 1]]),
                u16::from_le_bytes([rec[c + 2], rec[c + 3]]),
                u16::from_le_bytes([rec[c + 4], rec[c + 5]]),
            )
        }))
    }

    /// NIR column, or `None` if the format has no NIR field.
    pub fn nir(&self) -> Option<impl Iterator<Item = u16> + '_> {
        let n = self.layout.nir?;
        Some(
            self.records()
                .map(move |rec| u16::from_le_bytes([rec[n], rec[n + 1]])),
        )
    }

    fn records(&self) -> impl Iterator<Item = &[u8]> + '_ {
        self.bytes.chunks_exact(self.layout.record_len)
    }

    fn i32_column(&self, field_offset: usize) -> I32Column<'_> {
        I32Column {
            bytes: &self.bytes,
            stride: self.layout.record_len,
            field: field_offset,
            pos: 0,
        }
    }

    fn u16_column(&self, field_offset: usize) -> U16Column<'_> {
        U16Column {
            bytes: &self.bytes,
            stride: self.layout.record_len,
            field: field_offset,
            pos: 0,
        }
    }
}

/// Iterator over points in a [Points], yielding owned [Point] values.
///
/// Returned by [`Points::iter`]. Each call to `next` decodes one record
/// through [`raw::Point::read_from`] + [`Point::new`], matching the cost and
/// semantics of [`Reader::read_points_into`](crate::Reader::read_points_into).
#[allow(missing_debug_implementations)]
pub struct PointIter<'a> {
    cursor: Cursor<&'a [u8]>,
    format: &'a Format,
    transforms: &'a Vector<Transform>,
    remaining: usize,
}

impl Iterator for PointIter<'_> {
    type Item = Result<Point>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(
            raw::Point::read_from(&mut self.cursor, self.format)
                .map(|raw_point| Point::new(raw_point, self.transforms)),
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for PointIter<'_> {}

#[derive(Debug)]
struct I32Column<'a> {
    bytes: &'a [u8],
    stride: usize,
    field: usize,
    pos: usize,
}

impl Iterator for I32Column<'_> {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        let start = self.pos + self.field;
        if start + 4 > self.bytes.len() {
            return None;
        }
        let v = i32::from_le_bytes([
            self.bytes[start],
            self.bytes[start + 1],
            self.bytes[start + 2],
            self.bytes[start + 3],
        ]);
        self.pos += self.stride;
        Some(v)
    }
}

#[derive(Debug)]
struct U16Column<'a> {
    bytes: &'a [u8],
    stride: usize,
    field: usize,
    pos: usize,
}

impl Iterator for U16Column<'_> {
    type Item = u16;
    fn next(&mut self) -> Option<u16> {
        let start = self.pos + self.field;
        if start + 2 > self.bytes.len() {
            return None;
        }
        let v = u16::from_le_bytes([self.bytes[start], self.bytes[start + 1]]);
        self.pos += self.stride;
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        point::Format,
        raw::{
            self,
            point::{Flags, ScanAngle, Waveform},
        },
        Transform, Vector,
    };

    fn default_transforms() -> Vector<Transform> {
        Vector {
            x: Transform::default(),
            y: Transform::default(),
            z: Transform::default(),
        }
    }

    fn build_raw_point(format: &Format, i: i32) -> raw::Point {
        let flags = if format.is_extended {
            // return_number=1, number_of_returns=2, scanner_channel=1,
            // synthetic=1, classification=5.
            Flags::ThreeByte((2 << 4) | 1, (1 << 4) | 1, 5)
        } else {
            // two-byte: return_number=1, number_of_returns=2; b byte:
            // classification=3, synthetic=1 (bit 5).
            Flags::TwoByte((2 << 3) | 1, 3 | 0b0010_0000)
        };
        let scan_angle = if format.is_extended {
            ScanAngle::Scaled(1500) // 9.0 degrees
        } else {
            ScanAngle::Rank(-12)
        };
        raw::Point {
            x: i,
            y: i + 1,
            z: i + 2,
            intensity: 1000 + i as u16,
            flags,
            scan_angle,
            user_data: 7,
            point_source_id: 42,
            gps_time: if format.has_gps_time {
                Some(1234.5 + f64::from(i))
            } else {
                None
            },
            color: if format.has_color {
                Some(crate::Color {
                    red: 100 + i as u16,
                    green: 200 + i as u16,
                    blue: 300 + i as u16,
                })
            } else {
                None
            },
            waveform: if format.has_waveform {
                Some(Waveform::default())
            } else {
                None
            },
            nir: if format.has_nir { Some(4242) } else { None },
            extra_bytes: vec![0u8; format.extra_bytes as usize],
        }
    }

    fn build_points_for_format(format: Format) -> Points {
        let n = 5i32;
        let mut buf: Vec<u8> = Vec::new();
        for i in 0..n {
            let rp = build_raw_point(&format, i);
            rp.write_to(&mut buf, &format).unwrap();
        }
        let mut points = Points::new(format, default_transforms());
        let slab = points.resize_for(n as usize);
        slab.copy_from_slice(&buf);
        points
    }

    fn check_format(format: Format) {
        let points = build_points_for_format(format);
        assert_eq!(points.len(), 5);

        // Columns match per-point decode.
        let xs_col: Vec<i32> = points.x_raw().collect();
        let ys_col: Vec<i32> = points.y_raw().collect();
        let zs_col: Vec<i32> = points.z_raw().collect();
        let int_col: Vec<u16> = points.intensity().collect();
        let cls_col: Vec<u8> = points.classification().collect();
        let rn_col: Vec<u8> = points.return_number().collect();
        let nr_col: Vec<u8> = points.number_of_returns().collect();
        let sa_col: Vec<f32> = points.scan_angle_degrees().collect();
        let ud_col: Vec<u8> = points.user_data().collect();
        let ps_col: Vec<u16> = points.point_source_id().collect();

        for (i, p) in points.iter().enumerate() {
            let p = p.unwrap();
            assert_eq!(xs_col[i], i as i32);
            assert_eq!(ys_col[i], i as i32 + 1);
            assert_eq!(zs_col[i], i as i32 + 2);
            assert_eq!(int_col[i], 1000 + i as u16);
            assert_eq!(rn_col[i], 1);
            assert_eq!(nr_col[i], 2);
            assert_eq!(ud_col[i], 7);
            assert_eq!(ps_col[i], 42);
            assert_eq!(rn_col[i], p.return_number);
            assert_eq!(nr_col[i], p.number_of_returns);
            assert_eq!(ud_col[i], p.user_data);
            assert_eq!(ps_col[i], p.point_source_id);
            if format.is_extended {
                assert_eq!(cls_col[i], 5);
                assert!((sa_col[i] - 9.0).abs() < 1e-3);
            } else {
                assert_eq!(cls_col[i], 3);
                assert!((sa_col[i] - (-12.0)).abs() < 1e-3);
            }
            if format.has_gps_time {
                assert_eq!(p.gps_time, Some(1234.5 + f64::from(i as i32)));
            } else {
                assert_eq!(p.gps_time, None);
            }
            if format.has_color {
                let c = p.color.unwrap();
                assert_eq!(
                    (c.red, c.green, c.blue),
                    (100 + i as u16, 200 + i as u16, 300 + i as u16)
                );
            } else {
                assert!(p.color.is_none());
            }
            if format.has_nir {
                assert_eq!(p.nir, Some(4242));
            } else {
                assert_eq!(p.nir, None);
            }
        }

        if format.has_gps_time {
            let gps_col: Vec<f64> = points.gps_time().unwrap().collect();
            let gps_row: Vec<f64> = points
                .iter()
                .map(|p| p.unwrap().gps_time.unwrap())
                .collect();
            assert_eq!(gps_col, gps_row);
        } else {
            assert!(points.gps_time().is_none());
        }
        if format.has_color {
            let rgb_col: Vec<(u16, u16, u16)> = points.rgb().unwrap().collect();
            let rgb_row: Vec<(u16, u16, u16)> = points
                .iter()
                .map(|p| {
                    let c = p.unwrap().color.unwrap();
                    (c.red, c.green, c.blue)
                })
                .collect();
            assert_eq!(rgb_col, rgb_row);
        }
        if format.has_nir {
            let nir_col: Vec<u16> = points.nir().unwrap().collect();
            let nir_row: Vec<u16> = points.iter().map(|p| p.unwrap().nir.unwrap()).collect();
            assert_eq!(nir_col, nir_row);
        }
    }

    #[test]
    fn format_0() {
        check_format(Format::new(0).unwrap());
    }

    #[test]
    fn format_1() {
        check_format(Format::new(1).unwrap());
    }

    #[test]
    fn format_2() {
        check_format(Format::new(2).unwrap());
    }

    #[test]
    fn format_3() {
        check_format(Format::new(3).unwrap());
    }

    #[test]
    fn format_6() {
        check_format(Format::new(6).unwrap());
    }

    #[test]
    fn format_7() {
        check_format(Format::new(7).unwrap());
    }

    #[test]
    fn format_8() {
        check_format(Format::new(8).unwrap());
    }

    #[test]
    fn record_len_matches_format_len() {
        for n in [0u8, 1, 2, 3, 6, 7, 8] {
            let f = Format::new(n).unwrap();
            let points = Points::new(f, default_transforms());
            assert_eq!(points.record_len(), f.len() as usize, "format {n}");
        }
    }

    #[test]
    fn from_raw_bytes_valid() {
        let format = Format::new(1).unwrap();
        let transforms = default_transforms();
        let record_len = format.len() as usize;
        let mut buf = Vec::new();
        for i in 0..3 {
            let rp = build_raw_point(&format, i);
            rp.write_to(&mut buf, &format).unwrap();
        }
        assert_eq!(buf.len(), record_len * 3);
        let points = Points::from_raw_bytes(format, transforms, buf).unwrap();
        assert_eq!(points.len(), 3);
        let xs: Vec<i32> = points.x_raw().collect();
        assert_eq!(xs, vec![0, 1, 2]);
    }

    #[test]
    fn from_raw_bytes_rejects_bad_length() {
        let format = Format::new(0).unwrap();
        let transforms = default_transforms();
        let record_len = format.len() as usize;
        let buf = vec![0u8; record_len * 2 + 1];
        assert!(Points::from_raw_bytes(format, transforms, buf).is_err());
    }

    #[test]
    fn from_raw_bytes_accepts_empty() {
        let format = Format::new(0).unwrap();
        let transforms = default_transforms();
        let points = Points::from_raw_bytes(format, transforms, Vec::new()).unwrap();
        assert!(points.is_empty());
    }

    #[test]
    fn resize_for_and_fill() {
        let format = Format::new(1).unwrap();
        let transforms = default_transforms();
        let mut points = Points::new(format, transforms);
        let mut buf = Vec::new();
        for i in 0..2 {
            build_raw_point(&format, i)
                .write_to(&mut buf, &format)
                .unwrap();
        }
        let slab = points.resize_for(2);
        slab.copy_from_slice(&buf);
        assert_eq!(points.len(), 2);
        let xs: Vec<i32> = points.x_raw().collect();
        assert_eq!(xs, vec![0, 1]);
    }

    #[test]
    fn iter_matches_columns() {
        let format = Format::new(7).unwrap();
        let points = build_points_for_format(format);
        let owned: Vec<Point> = points.iter().map(Result::unwrap).collect();
        let xs_col: Vec<f64> = points.x().collect();
        let ints: Vec<u16> = points.intensity().collect();
        assert_eq!(owned.len(), points.len());
        for (i, p) in owned.iter().enumerate() {
            assert!((p.x - xs_col[i]).abs() < 1e-12);
            assert_eq!(p.intensity, ints[i]);
        }
    }
}
