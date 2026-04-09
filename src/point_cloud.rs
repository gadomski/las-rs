//! A byte-slab point cloud, for high-throughput bulk reads.
//!
//! [PointCloud] holds a single contiguous `Vec<u8>` of decompressed LAS point
//! records in their on-disk layout. It is the "bytes-out" counterpart to the
//! existing [`Vec<Point>`](crate::Point) iteration API, and is intended for the
//! throughput case: decoding millions of points at a time, or computing
//! column-oriented statistics (bounds, means, histograms) without paying the
//! per-point [Point](crate::Point) materialization cost.
//!
//! The existing [Point](crate::Point) / [Reader::read_points_into](crate::Reader::read_points_into)
//! API is unchanged and remains the best choice for scripts and one-off
//! operations. [PointCloud] is additive.
//!
//! # Example
//!
//! ```
//! use las::{Reader, PointCloud};
//!
//! let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
//! let cloud = reader.read_all_into_cloud().unwrap();
//!
//! // Row-oriented: iterate points.
//! for p in cloud.iter().take(3) {
//!     println!("{:.3} {:.3} {:.3} intensity={}", p.x(), p.y(), p.z(), p.intensity());
//! }
//!
//! // Column-oriented: one pass over the x column.
//! let min_x = cloud.x().fold(f64::INFINITY, f64::min);
//! let max_x = cloud.x().fold(f64::NEG_INFINITY, f64::max);
//! assert!(min_x <= max_x);
//! ```

use crate::{point::Format, Transform, Vector};

/// Per-format byte offsets for fields that live at a format-dependent position
/// within a record.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct Offsets {
    /// Offset of the `user_data` byte.
    user_data: usize,
    /// Offset of the scan angle field (i8 for legacy, i16 for extended).
    scan_angle: usize,
    /// Offset of the `point_source_id` u16.
    point_source_id: usize,
    /// Offset of the gps_time f64, if the format has one.
    gps_time: Option<usize>,
    /// Offset of the RGB triple, if the format has color.
    rgb: Option<usize>,
    /// Offset of the waveform packet (29 bytes), if the format has waveform.
    waveform: Option<usize>,
    /// Offset of the NIR u16, if the format has NIR.
    nir: Option<usize>,
    /// Offset of the extra_bytes region.
    extra_bytes: usize,
    /// Total record length in bytes.
    record_len: usize,
}

impl Offsets {
    fn for_format(format: &Format) -> Self {
        // Layout mirrors `raw::Point::read_from`:
        //   0..4   x (i32)
        //   4..8   y (i32)
        //   8..12  z (i32)
        //   12..14 intensity (u16)
        //   14..   flags (1 or 2 bytes for legacy, 3 bytes for extended)
        //   then   legacy:   scan_angle (i8), user_data (u8)
        //          extended: user_data (u8), scan_angle (i16)
        //   then   point_source_id (u16)
        //   then   optional gps_time, color, waveform, nir
        //   then   extra_bytes
        let mut off = 14usize;
        if format.is_extended {
            off += 3; // flags (3 bytes)
        } else {
            off += 2; // flags (2 bytes)
        }
        let (user_data, scan_angle) = if format.is_extended {
            // extended: user_data then i16 scan angle
            let ud = off;
            let sa = off + 1;
            off += 3;
            (ud, sa)
        } else {
            // legacy: i8 scan angle then user_data
            let sa = off;
            let ud = off + 1;
            off += 2;
            (ud, sa)
        };
        let point_source_id = off;
        off += 2;
        let gps_time = if format.has_gps_time {
            let g = off;
            off += 8;
            Some(g)
        } else {
            None
        };
        let rgb = if format.has_color {
            let c = off;
            off += 6;
            Some(c)
        } else {
            None
        };
        let waveform = if format.has_waveform {
            let w = off;
            off += 29;
            Some(w)
        } else {
            None
        };
        let nir = if format.has_nir {
            let n = off;
            off += 2;
            Some(n)
        } else {
            None
        };
        let extra_bytes = off;
        off += format.extra_bytes as usize;
        Offsets {
            user_data,
            scan_angle,
            point_source_id,
            gps_time,
            rgb,
            waveform,
            nir,
            extra_bytes,
            record_len: off,
        }
    }
}

/// A set of decompressed LAS point records held as one contiguous byte slab.
///
/// `PointCloud` mirrors the on-disk layout for a specific point format. It is
/// filled by [Reader::read_into_cloud](crate::Reader::read_into_cloud) or
/// [Reader::read_all_into_cloud](crate::Reader::read_all_into_cloud). Individual
/// points can be borrowed as [PointRef] without allocating, and columns can be
/// iterated field-by-field via [PointCloud::x], [PointCloud::intensity], and so
/// on.
///
/// # Example
///
/// ```
/// use las::{Reader, PointCloud};
///
/// let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
/// let mut cloud = PointCloud::new(
///     *reader.header().point_format(),
///     *reader.header().transforms(),
/// );
/// let n = reader.read_into_cloud(&mut cloud, 10).unwrap();
/// assert_eq!(n, 10);
/// assert_eq!(cloud.len(), 10);
/// ```
#[derive(Clone, Debug)]
pub struct PointCloud {
    bytes: Vec<u8>,
    format: Format,
    transforms: Vector<Transform>,
    offsets: Offsets,
}

impl PointCloud {
    /// Creates an empty point cloud for the given format and coordinate
    /// transforms.
    ///
    /// The format and transforms are cached for per-point accessor dispatch,
    /// and must match the file this cloud will be filled from. In typical
    /// usage, pass `*reader.header().point_format()` and
    /// `*reader.header().transforms()`.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Reader, PointCloud};
    /// let reader = Reader::from_path("tests/data/autzen.las").unwrap();
    /// let cloud = PointCloud::new(
    ///     *reader.header().point_format(),
    ///     *reader.header().transforms(),
    /// );
    /// assert!(cloud.is_empty());
    /// ```
    pub fn new(format: Format, transforms: Vector<Transform>) -> Self {
        let offsets = Offsets::for_format(&format);
        PointCloud {
            bytes: Vec::new(),
            format,
            transforms,
            offsets,
        }
    }

    /// Returns the number of points currently held.
    pub fn len(&self) -> usize {
        if self.offsets.record_len == 0 {
            0
        } else {
            self.bytes.len() / self.offsets.record_len
        }
    }

    /// Returns true if this cloud contains no points.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the point format this cloud was built for.
    pub fn format(&self) -> &Format {
        &self.format
    }

    /// Returns the coordinate transforms this cloud was built with.
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

    /// Returns the record length in bytes for this cloud's format.
    pub fn record_len(&self) -> usize {
        self.offsets.record_len
    }

    /// Borrows a single point by index.
    ///
    /// # Panics
    ///
    /// Panics if `i >= self.len()`.
    pub fn point(&self, i: usize) -> PointRef<'_> {
        let rec = self.offsets.record_len;
        let start = i * rec;
        PointRef {
            bytes: &self.bytes[start..start + rec],
            format: &self.format,
            transforms: &self.transforms,
            offsets: &self.offsets,
        }
    }

    /// Iterates points row-by-row.
    pub fn iter(&self) -> PointRefIter<'_> {
        PointRefIter {
            cloud: self,
            index: 0,
            len: self.len(),
        }
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
        self.iter().map(move |p| {
            if is_extended {
                p.bytes[16]
            } else {
                p.bytes[15] & 0b0001_1111
            }
        })
    }

    /// Return number column.
    pub fn return_number(&self) -> impl Iterator<Item = u8> + '_ {
        let is_extended = self.format.is_extended;
        self.iter().map(move |p| {
            if is_extended {
                p.bytes[14] & 15
            } else {
                p.bytes[14] & 7
            }
        })
    }

    /// Number-of-returns column.
    pub fn number_of_returns(&self) -> impl Iterator<Item = u8> + '_ {
        let is_extended = self.format.is_extended;
        self.iter().map(move |p| {
            if is_extended {
                (p.bytes[14] >> 4) & 15
            } else {
                (p.bytes[14] >> 3) & 7
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
        let sa_off = self.offsets.scan_angle;
        self.iter().map(move |p| {
            if is_extended {
                let raw = i16::from_le_bytes([p.bytes[sa_off], p.bytes[sa_off + 1]]);
                f32::from(raw) * 0.006
            } else {
                f32::from(p.bytes[sa_off] as i8)
            }
        })
    }

    /// User data byte column.
    pub fn user_data(&self) -> impl Iterator<Item = u8> + '_ {
        let ud = self.offsets.user_data;
        self.iter().map(move |p| p.bytes[ud])
    }

    /// Point source ID column.
    pub fn point_source_id(&self) -> impl Iterator<Item = u16> + '_ {
        let ps = self.offsets.point_source_id;
        self.iter()
            .map(move |p| u16::from_le_bytes([p.bytes[ps], p.bytes[ps + 1]]))
    }

    /// GPS time column, or `None` if the format has no gps_time field.
    pub fn gps_time(&self) -> Option<impl Iterator<Item = f64> + '_> {
        let g = self.offsets.gps_time?;
        Some(self.iter().map(move |p| {
            f64::from_le_bytes([
                p.bytes[g],
                p.bytes[g + 1],
                p.bytes[g + 2],
                p.bytes[g + 3],
                p.bytes[g + 4],
                p.bytes[g + 5],
                p.bytes[g + 6],
                p.bytes[g + 7],
            ])
        }))
    }

    /// RGB column, or `None` if the format has no color.
    pub fn rgb(&self) -> Option<impl Iterator<Item = (u16, u16, u16)> + '_> {
        let c = self.offsets.rgb?;
        Some(self.iter().map(move |p| {
            (
                u16::from_le_bytes([p.bytes[c], p.bytes[c + 1]]),
                u16::from_le_bytes([p.bytes[c + 2], p.bytes[c + 3]]),
                u16::from_le_bytes([p.bytes[c + 4], p.bytes[c + 5]]),
            )
        }))
    }

    /// NIR column, or `None` if the format has no NIR field.
    pub fn nir(&self) -> Option<impl Iterator<Item = u16> + '_> {
        let n = self.offsets.nir?;
        Some(
            self.iter()
                .map(move |p| u16::from_le_bytes([p.bytes[n], p.bytes[n + 1]])),
        )
    }

    /// Returns a mutable view of the underlying byte buffer, resized to hold
    /// exactly `n` points. Intended for internal use by the reader fast paths.
    pub(crate) fn resize_for(&mut self, n: usize) -> &mut [u8] {
        let new_len = n.checked_mul(self.offsets.record_len).expect("overflow");
        self.bytes.resize(new_len, 0u8);
        &mut self.bytes
    }

    fn i32_column(&self, field_offset: usize) -> I32Column<'_> {
        I32Column {
            bytes: &self.bytes,
            stride: self.offsets.record_len,
            field: field_offset,
            pos: 0,
        }
    }

    fn u16_column(&self, field_offset: usize) -> U16Column<'_> {
        U16Column {
            bytes: &self.bytes,
            stride: self.offsets.record_len,
            field: field_offset,
            pos: 0,
        }
    }
}

/// A zero-copy, borrowed view of a single point within a [PointCloud].
#[derive(Debug, Clone, Copy)]
pub struct PointRef<'a> {
    bytes: &'a [u8],
    format: &'a Format,
    transforms: &'a Vector<Transform>,
    offsets: &'a Offsets,
}

impl<'a> PointRef<'a> {
    /// Raw i32 x.
    pub fn x_raw(&self) -> i32 {
        i32::from_le_bytes([self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]])
    }

    /// Raw i32 y.
    pub fn y_raw(&self) -> i32 {
        i32::from_le_bytes([self.bytes[4], self.bytes[5], self.bytes[6], self.bytes[7]])
    }

    /// Raw i32 z.
    pub fn z_raw(&self) -> i32 {
        i32::from_le_bytes([self.bytes[8], self.bytes[9], self.bytes[10], self.bytes[11]])
    }

    /// World x coordinate.
    pub fn x(&self) -> f64 {
        self.transforms.x.direct(self.x_raw())
    }

    /// World y coordinate.
    pub fn y(&self) -> f64 {
        self.transforms.y.direct(self.y_raw())
    }

    /// World z coordinate.
    pub fn z(&self) -> f64 {
        self.transforms.z.direct(self.z_raw())
    }

    /// Intensity.
    pub fn intensity(&self) -> u16 {
        u16::from_le_bytes([self.bytes[12], self.bytes[13]])
    }

    /// Classification code.
    pub fn classification(&self) -> u8 {
        if self.format.is_extended {
            self.bytes[16]
        } else {
            self.bytes[15] & 0b0001_1111
        }
    }

    /// Return number.
    pub fn return_number(&self) -> u8 {
        if self.format.is_extended {
            self.bytes[14] & 15
        } else {
            self.bytes[14] & 7
        }
    }

    /// Number of returns for the pulse.
    pub fn number_of_returns(&self) -> u8 {
        if self.format.is_extended {
            (self.bytes[14] >> 4) & 15
        } else {
            (self.bytes[14] >> 3) & 7
        }
    }

    /// Scan angle, in degrees.
    pub fn scan_angle_degrees(&self) -> f32 {
        let sa = self.offsets.scan_angle;
        if self.format.is_extended {
            let raw = i16::from_le_bytes([self.bytes[sa], self.bytes[sa + 1]]);
            f32::from(raw) * 0.006
        } else {
            f32::from(self.bytes[sa] as i8)
        }
    }

    /// User data byte.
    pub fn user_data(&self) -> u8 {
        self.bytes[self.offsets.user_data]
    }

    /// Point source ID.
    pub fn point_source_id(&self) -> u16 {
        let ps = self.offsets.point_source_id;
        u16::from_le_bytes([self.bytes[ps], self.bytes[ps + 1]])
    }

    /// GPS time, if the format has one.
    pub fn gps_time(&self) -> Option<f64> {
        let g = self.offsets.gps_time?;
        Some(f64::from_le_bytes([
            self.bytes[g],
            self.bytes[g + 1],
            self.bytes[g + 2],
            self.bytes[g + 3],
            self.bytes[g + 4],
            self.bytes[g + 5],
            self.bytes[g + 6],
            self.bytes[g + 7],
        ]))
    }

    /// RGB triple, if the format has color.
    pub fn rgb(&self) -> Option<(u16, u16, u16)> {
        let c = self.offsets.rgb?;
        Some((
            u16::from_le_bytes([self.bytes[c], self.bytes[c + 1]]),
            u16::from_le_bytes([self.bytes[c + 2], self.bytes[c + 3]]),
            u16::from_le_bytes([self.bytes[c + 4], self.bytes[c + 5]]),
        ))
    }

    /// Near-infrared value, if the format has one.
    pub fn nir(&self) -> Option<u16> {
        let n = self.offsets.nir?;
        Some(u16::from_le_bytes([self.bytes[n], self.bytes[n + 1]]))
    }

    /// Whether this point is synthetic.
    pub fn is_synthetic(&self) -> bool {
        if self.format.is_extended {
            self.bytes[15] & 1 == 1
        } else {
            (self.bytes[15] >> 5) & 1 == 1
        }
    }

    /// Whether this point is a model key-point.
    pub fn is_key_point(&self) -> bool {
        if self.format.is_extended {
            self.bytes[15] & 2 == 2
        } else {
            (self.bytes[15] >> 6) & 1 == 1
        }
    }

    /// Whether this point is withheld.
    pub fn is_withheld(&self) -> bool {
        if self.format.is_extended {
            self.bytes[15] & 4 == 4
        } else {
            (self.bytes[15] >> 7) & 1 == 1
        }
    }

    /// Whether this point is in an overlap region.
    ///
    /// For legacy formats this is signalled by classification code 12; for
    /// extended formats it is bit 3 of the second flags byte.
    pub fn is_overlap(&self) -> bool {
        if self.format.is_extended {
            self.bytes[15] & 8 == 8
        } else {
            self.bytes[15] & 0b1111 == 12
        }
    }

    /// Scanner channel (extended formats only; 0 for legacy).
    pub fn scanner_channel(&self) -> u8 {
        if self.format.is_extended {
            (self.bytes[15] >> 4) & 3
        } else {
            0
        }
    }

    /// Raw bytes of this record.
    pub fn raw_bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Iterator over points in a [PointCloud].
#[derive(Debug)]
pub struct PointRefIter<'a> {
    cloud: &'a PointCloud,
    index: usize,
    len: usize,
}

impl<'a> Iterator for PointRefIter<'a> {
    type Item = PointRef<'a>;
    fn next(&mut self) -> Option<PointRef<'a>> {
        if self.index >= self.len {
            None
        } else {
            let p = self.cloud.point(self.index);
            self.index += 1;
            Some(p)
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = self.len - self.index;
        (rem, Some(rem))
    }
}

impl ExactSizeIterator for PointRefIter<'_> {}

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
    use std::io::Cursor;

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
            // synthetic=1, key_point=0, withheld=0, overlap=0,
            // scan_direction=0, edge_of_flight_line=0, classification=5.
            Flags::ThreeByte((2 << 4) | 1, (1 << 4) | 1, 5)
        } else {
            // two-byte: return_number=1, number_of_returns=2,
            // scan_direction=0, edge_of_flight=0; b byte: classification=3,
            // synthetic=1 (bit 5).
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

    fn build_cloud_for_format(format: Format) -> PointCloud {
        let n = 5i32;
        let mut buf: Vec<u8> = Vec::new();
        for i in 0..n {
            let rp = build_raw_point(&format, i);
            rp.write_to(&mut buf, &format).unwrap();
        }
        let mut cloud = PointCloud::new(format, default_transforms());
        let slab = cloud.resize_for(n as usize);
        slab.copy_from_slice(&buf);
        cloud
    }

    fn check_format(format: Format) {
        let cloud = build_cloud_for_format(format);
        assert_eq!(cloud.len(), 5);
        for i in 0..5usize {
            let p = cloud.point(i);
            assert_eq!(p.x_raw(), i as i32, "x_raw format {format:?}");
            assert_eq!(p.y_raw(), i as i32 + 1);
            assert_eq!(p.z_raw(), i as i32 + 2);
            assert_eq!(p.intensity(), 1000 + i as u16);
            assert_eq!(p.return_number(), 1);
            assert_eq!(p.number_of_returns(), 2);
            assert_eq!(p.user_data(), 7);
            assert_eq!(p.point_source_id(), 42);
            if format.is_extended {
                assert_eq!(p.classification(), 5);
                assert_eq!(p.scanner_channel(), 1);
                assert!(p.is_synthetic());
                assert!(!p.is_key_point());
                assert!(!p.is_withheld());
                assert!((p.scan_angle_degrees() - 9.0).abs() < 1e-3);
            } else {
                assert_eq!(p.classification(), 3);
                assert_eq!(p.scanner_channel(), 0);
                assert!(p.is_synthetic());
                assert!((p.scan_angle_degrees() - (-12.0)).abs() < 1e-3);
            }
            if format.has_gps_time {
                assert_eq!(p.gps_time(), Some(1234.5 + f64::from(i as i32)));
            } else {
                assert_eq!(p.gps_time(), None);
            }
            if format.has_color {
                assert_eq!(
                    p.rgb(),
                    Some((100 + i as u16, 200 + i as u16, 300 + i as u16))
                );
            } else {
                assert_eq!(p.rgb(), None);
            }
            if format.has_nir {
                assert_eq!(p.nir(), Some(4242));
            } else {
                assert_eq!(p.nir(), None);
            }
        }
        // Column iterators match per-point accessors.
        let xs_col: Vec<i32> = cloud.x_raw().collect();
        let xs_row: Vec<i32> = cloud.iter().map(|p| p.x_raw()).collect();
        assert_eq!(xs_col, xs_row);
        let int_col: Vec<u16> = cloud.intensity().collect();
        let int_row: Vec<u16> = cloud.iter().map(|p| p.intensity()).collect();
        assert_eq!(int_col, int_row);
        if format.has_gps_time {
            let gps_col: Vec<f64> = cloud.gps_time().unwrap().collect();
            let gps_row: Vec<f64> = cloud.iter().map(|p| p.gps_time().unwrap()).collect();
            assert_eq!(gps_col, gps_row);
        } else {
            assert!(cloud.gps_time().is_none());
        }
        if format.has_color {
            let rgb_col: Vec<(u16, u16, u16)> = cloud.rgb().unwrap().collect();
            let rgb_row: Vec<(u16, u16, u16)> = cloud.iter().map(|p| p.rgb().unwrap()).collect();
            assert_eq!(rgb_col, rgb_row);
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
            let cloud = PointCloud::new(f, default_transforms());
            assert_eq!(cloud.record_len(), f.len() as usize, "format {n}");
        }
    }

    #[test]
    fn round_trips_through_cursor_of_bytes() {
        let format = Format::new(1).unwrap();
        let cloud = build_cloud_for_format(format);
        let bytes = cloud.raw_bytes().to_vec();
        let mut cursor = Cursor::new(bytes);
        for i in 0..5usize {
            let rp = raw::Point::read_from(&mut cursor, &format).unwrap();
            assert_eq!(rp.x, i as i32);
            assert_eq!(rp.intensity, 1000 + i as u16);
        }
    }
}
