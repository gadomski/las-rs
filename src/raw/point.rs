//! Defines raw las points and some enums required to handle the various point formats.

use {Color, Result};
use point::{Classification, Error, Format, ScanDirection};
use std::io::{Read, Write};

const SCAN_ANGLE_SCALE_FACTOR: f32 = 0.006;
const OVERLAP_CLASSIFICATION_CODE: u8 = 12;

/// A raw point.
///
/// The documentation for struct members is taken directly from the las 1.4 spec.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Point {
    /// The X, Y, and Z values are stored as long integers.
    ///
    /// The X, Y, and Z values are used in conjunction with the scale values and the offset values
    /// to determine the coordinate for each point as described in the Public Header Block section.
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
    #[allow(missing_docs)]
    pub z: i32,

    /// The intensity value is the integer representation of the pulse return magnitude.
    ///
    /// This value is optional and system specific. However, it should always be included if
    /// available. Intensity, when included, is always normalized to a 16 bit, unsigned value by
    /// multiplying the value by 65,536/(intensity dynamic range of the sensor). For example, if
    /// the dynamic range of the sensor is 10 bits, the scaling value would be (65,536/1,024). If
    /// intensity is not included, this value must be set to zero. This normalization is required
    /// to ensure that data from different sensors can be correctly merged.
    pub intensity: u16,

    /// Flags can either be one or two bytes.
    ///
    /// # One byte
    ///
    /// Please note that the following four fields (Return Number, Number of Returns, Scan
    /// Direction Flag and Edge of Flight Line) are bit fields within a single byte.
    ///
    /// ## Return Number
    ///
    /// The Return Number is the pulse return number for a given output pulse. A given output laser
    /// pulse can have many returns, and they must be marked in sequence of return. The first
    /// return will have a Return Number of one, the second a Return Number of two, and so on up to
    /// five returns.
    ///
    /// ## Number of Returns (given pulse)
    ///
    /// The Number of Returns is the total number of returns for a given pulse. For example, a
    /// laser data point may be return two (Return Number) within a total number of five returns.
    ///
    /// ## Scan Direction Flag
    ///
    /// The Scan Direction Flag denotes the direction at which the scanner mirror was traveling at
    /// the time of the output pulse. A bit value of 1 is a positive scan direction, and a bit
    /// value of 0 is a negative scan direction (where positive scan direction is a scan moving
    /// from the left side of the in-track direction to the right side and negative the opposite).
    ///
    /// ## Edge of Flight Line
    ///
    /// The Edge of Flight Line data bit has a value of 1 only when the point is at the end of a
    /// scan. It is the last point on a given scan line before it changes direction.
    ///
    /// # Two bytes
    ///
    /// Note that the following five fields (Return Number, Number of Returns, Classification
    /// Flags, Scan Direction Flag and Edge of Flight Line) are bit fields, encoded into two bytes.
    /// ## Return Number
    ///
    /// The Return Number is the pulse return number for a given output pulse. A given output laser
    /// pulse can have many returns, and they must be marked in sequence of return. The first
    /// return will have a Return Number of one, the second a Return Number of two, and so on up to
    /// fifteen returns. The Return Number must be between 1 and the Number of Returns, inclusive.
    ///
    /// ## Number of Returns (given pulse)
    ///
    /// The Number of Returns is the total number of returns for a given pulse. For example, a
    /// laser data point may be return two (Return Number) within a total number of up to fifteen
    /// returns.
    ///
    /// ## Classification Flags
    ///
    /// Classification flags are used to indicate special characteristics associated with the
    /// point. The bit definitions are:
    ///
    /// | Bit | Field name | Description |
    /// | --- | ---------- | ----------- |
    /// | 0 | Synthetic | If set then this point was created by a technique other than LIDAR collection such as digitized from a photogrammetric stereo model or by traversing a waveform. |
    /// | 1 | Key-point | If set, this point is considered to be a model key-point and thus generally should not be withheld in a thinning algorithm. |
    /// | 2 | Withheld | If set, this point should not be included in processing (synonymous with Deleted). |
    /// | 3 | Overlap | If set, this point is within the overlap region of two or more swaths or takes. Setting this bit is not mandatory (unless, of course, it is mandated by a particular delivery specification) but allows Classification of overlap points to be preserved. |
    ///
    /// Note that these bits are treated as flags and can be set or cleared in any combination. For
    /// example, a point with bits 0 and 1 both set to one and the Classification field set to 2
    /// would be a ground point that had been synthetically collected and marked as a model
    /// key-point.
    ///
    /// ## Scanner Channel
    ///
    /// Scanner Channel is used to indicate the channel (scanner head) of a multi- channel system.
    /// Channel 0 is used for single scanner systems. Up to four channels are supported (0-3).
    ///
    /// ## Scan Direction Flag
    ///
    /// The Scan Direction Flag denotes the direction at which the scanner mirror was traveling at
    /// the time of the output pulse. A bit value of 1 is a positive scan direction, and a bit
    /// value of 0 is a negative scan direction (where positive scan direction is a scan moving
    /// from the left side of the in-track direction to the right side and negative the opposite).
    ///
    /// ## Edge of Flight Line
    ///
    /// The Edge of Flight Line data bit has a value of 1 only when the point is at the end of a
    /// scan. It is the last point on a given scan line before it changes direction or the mirror
    /// facet changes. Note that this field has no meaning for 360° Field of View scanners (such as
    /// Mobile LIDAR scanners) and should not be set.
    ///
    /// # Classification
    ///
    /// This field also holds the “class” attributes of a point.
    ///
    /// If a point has never been classified, this byte must be set to zero. In some point formats,
    /// the format for classification is a bit encoded field with the lower five bits used for the
    /// class and the three high bits used for flags. In others, the whole byte is used for
    /// classes.
    ///
    /// # Bit field encoding for point data record types 0 to 5
    ///
    /// | Bit | Field name | Description |
    /// | --- | ---------- | ----------- |
    /// | 0:4 | Classification | Standard ASPRS classification from 0 - 31 as defined in the classification table for legacy point formats |
    /// | 5 | Synthetic | If set then this point was created by a technique other than LIDAR collection such as digitized from a photogrammetric stereo model or by traversing a waveform. |
    /// | 6 | Key-point | If set, this point is considered to be a model key-point and thus generally should not be withheld in a thinning algorithm. |
    /// | 7 | Withheld | If set, this point should not be included in processing (synonymous with Deleted). |
    ///
    /// # ASPRS standard LiDAR point classes for point data record types 0 to 5
    ///
    /// | Classification value | Meaning |
    /// | -------------------- | ------- |
    /// | 0 | Created, never classified |
    /// | 1 | Unclassified |
    /// | 2 | Ground |
    /// | 3 | Low vegetation |
    /// | 4 | Medium vegetation |
    /// | 5 | High vegetation |
    /// | 6 | Building |
    /// | 7 | Low point (noise) |
    /// | 8 | Model key-point (mass point) |
    /// | 9 | Water |
    /// | 10 | Reserved |
    /// | 11 | Reserved |
    /// | 12 | Overlap points |
    /// | 13-31 | Reserved |
    ///
    /// # ASPRS standard LiDAR point classes for point data record types 6 to 10
    ///
    /// | Classification value | Meaning |
    /// | -------------------- | ------- |
    /// | 0 | Created, never classified |
    /// | 1 | Unclassified |
    /// | 2 | Ground |
    /// | 3 | Low vegetation |
    /// | 4 | Medium vegetation |
    /// | 5 | High vegetation |
    /// | 6 | Building |
    /// | 7 | Low point (noise) |
    /// | 8 | Model key-point (mass point) |
    /// | 9 | Water |
    /// | 10 | Rail |
    /// | 11 | Road surface |
    /// | 12 | Overlap points |
    /// | 13 | Wire - guard (shield) |
    /// | 14 | Wire - conductor (phase) |
    /// | 15 | Transmission tower |
    /// | 16 | Wire-structure connector (e.g. insulator) |
    /// | 17 | Bridge deck |
    /// | 17 | High noise |
    /// | 19-63 | Reserved |
    /// | 64-255 | Userdefinable |
    pub flags: Flags,

    /// The scan angle can be stored as rank or scaled.
    ///
    /// # Rank
    ///
    /// The Scan Angle Rank is a signed one-byte number with a valid range from - 90 to +90.
    ///
    /// The Scan Angle Rank is the angle (rounded to the nearest integer in the absolute value
    /// sense) at which the laser point was output from the laser system including the roll of the
    /// aircraft. The scan angle is within 1 degree of accuracy from +90 to –90 degrees. The scan
    /// angle is an angle based on 0 degrees being nadir, and –90 degrees to the left side of the
    /// aircraft in the direction of flight.
    ///
    /// # Scaled
    ///
    /// The Scan Angle is a signed short that represents the rotational position of the emitted
    /// laser pulse with respect to the vertical of the coordinate system of the data. Down in the
    /// data coordinate system is the 0.0 position. Each increment represents 0.006 degrees.
    /// Counter- Clockwise rotation, as viewed from the rear of the sensor, facing in the
    /// along-track (positive trajectory) direction, is positive. The maximum value in the positive
    /// sense is 30,000 (180 degrees which is up in the coordinate system of the data). The maximum
    /// value in the negative direction is -30.000 which is also directly up.
    pub scan_angle: ScanAngle,

    /// This field may be used at the user’s discretion.
    pub user_data: u8,

    /// This value indicates the file from which this point originated.
    ///
    /// Valid values for this field are 1 to 65,535 inclusive with zero being used for a special
    /// case discussed below. The numerical value corresponds to the File Source ID from which this
    /// point originated. Zero is reserved as a convenience to system implementers. A Point Source
    /// ID of zero implies that this point originated in this file. This implies that processing
    /// software should set the Point Source ID equal to the File Source ID of the file containing
    /// this point at some time during processing.
    pub point_source_id: u16,

    /// The GPS Time is the double floating point time tag value at which the point was acquired.
    ///
    /// It is GPS Week Time if the Global Encoding low bit is clear and Adjusted Standard GPS Time
    /// if the Global Encoding low bit is set (see Global Encoding in the Public Header Block
    /// description).
    pub gps_time: Option<f64>,

    /// The red, green, and blue image channels associated with this point.
    ///
    /// The Red, Green, Blue values should always be normalized to 16 bit values. For example, when
    /// encoding an 8 bit per channel pixel, multiply each channel value by 256 prior to storage in
    /// these fields. This normalization allows color values from different camera bit depths to be
    /// accurately merged.
    pub color: Option<Color>,

    #[allow(missing_docs)]
    pub waveform: Option<Waveform>,

    /// The NIR (near infrared) channel value associated with this point.
    pub nir: Option<u16>,

    #[allow(missing_docs)]
    pub extra_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(missing_docs)]
pub struct Waveform {
    /// This value plus 99 is the Record ID of the Waveform Packet Descriptor and indicates the
    /// User Defined Record that describes the waveform packet associated with this LIDAR point.
    ///
    /// Up to 255 different User Defined Records which describe the waveform packet are supported.
    /// A value of zero indicates that there is no waveform data associated with this LIDAR point
    /// record.
    pub wave_packet_descriptor_index: u8,

    /// The waveform packet data are stored in the LAS file in an Extended Variable Length Record
    /// or in an auxiliary WPD file.
    ///
    /// The Byte Offset represents the location of the start of this LIDAR points’ waveform packet
    /// within the waveform data variable length record (or external file) relative to the
    /// beginning of the Waveform Packet Data header. The absolute location of the beginning of
    /// this waveform packet relative to the beginning of the file is given by:
    ///
    /// > Start of Waveform Data Packet Record + Byte offset to Waveform Packet Data
    ///
    /// for waveform packets stored within the LAS file and
    ///
    /// > Byte offset to Waveform Packet Data
    ///
    /// for data stored in an auxiliary file
    pub byte_offset_to_waveform_data: u64,

    /// The size, in bytes, of the waveform packet associated with this return.
    ///
    /// Note that each waveform can be of a different size (even those with the same Waveform
    /// Packet Descriptor index) due to packet compression. Also note that waveform packets can be
    /// located only via the Byte offset to Waveform Packet Data value since there is no
    /// requirement that records be stored sequentially.
    pub waveform_packet_size_in_bytes: u32,

    /// The offset in picoseconds (10-12) from the first digitized value to the location within the
    /// waveform packet that the associated return pulse was detected.
    pub return_point_waveform_location: f32,

    /// These parameters define a parametric line equation for extrapolating points along the associated waveform.
    ///
    /// The position along the wave is given by:
    ///
    /// X = X0 + X(t)
    ///
    /// Y = Y0 + Y(t)
    ///
    /// Z = Z0 + Z(t)
    ///
    /// where X, Y and Z are the spatial position of the derived point, X0, Y0, Z0 are the position
    /// of the “anchor” point (the X, Y, Z locations from this point’s data record) and t is the
    /// time, in picoseconds, relative to the anchor point (i.e. t = zero at the anchor point). The
    /// units of X, Y and Z are the units of the coordinate systems of the LAS data. If the
    /// coordinate system is geographic, the horizontal units are decimal degrees and the vertical
    /// units are meters.
    pub x_t: f32,
    #[allow(missing_docs)]
    pub y_t: f32,
    #[allow(missing_docs)]
    pub z_t: f32,
}

/// Scan angle can be stored as a i8 (rank) or i16 (scaled).
#[derive(Clone, Copy, Debug)]
#[allow(missing_docs)]
pub enum ScanAngle {
    Rank(i8),
    Scaled(i16),
}

/// These flags hold information about point classification, return number, and more.
///
/// In point formats zero through five, two bytes are used to hold all of the information. Point
/// formats six through ten use an extra byte, to enable more return numbers, more classifications,
/// and more.
///
/// This structure captures those alternatives and provides an API to convert between the two
/// types. Two-byte flags can always be transformed to three-byte flags, but going from three-bytes
/// to two-bytes can fail if information would be lost.
///
/// ```
/// use las::raw::point::Flags;
/// let two_byte = Flags::TwoByte(0b00001001, 1);
/// assert_eq!(Flags::ThreeByte(0b00010001, 0, 1), two_byte.into());
///
/// // Two-byte flags can't handle this large of return numbers.
/// let three_byte = Flags::ThreeByte(0b10001000, 0, 1);
/// assert!(three_byte.to_two_bytes().is_err());
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Flags {
    /// Two byte flags, used for point formats zero through five.
    TwoByte(u8, u8),
    /// Three byte flags, used for point formats six through ten.
    ThreeByte(u8, u8, u8),
}

impl Point {
    /// Reads a raw point.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::raw::Point;
    /// use las::point::Format;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(1994)).unwrap();
    /// let point = Point::read_from(file, Format::new(1).unwrap()).unwrap();
    /// ```
    pub fn read_from<R: Read>(mut read: R, format: Format) -> Result<Point> {
        use byteorder::{LittleEndian, ReadBytesExt};
        use utils;

        let x = read.read_i32::<LittleEndian>()?;
        let y = read.read_i32::<LittleEndian>()?;
        let z = read.read_i32::<LittleEndian>()?;
        let intensity = read.read_u16::<LittleEndian>()?;
        let flags = if format.is_extended {
            Flags::ThreeByte(read.read_u8()?, read.read_u8()?, read.read_u8()?)
        } else {
            Flags::TwoByte(read.read_u8()?, read.read_u8()?)
        };
        let scan_angle = if format.is_extended {
            ScanAngle::Scaled(read.read_i16::<LittleEndian>()?)
        } else {
            ScanAngle::Rank(read.read_i8()?)
        };
        let user_data = read.read_u8()?;
        let point_source_id = read.read_u16::<LittleEndian>()?;
        let gps_time = if format.has_gps_time {
            utils::some_or_none_if_zero(read.read_f64::<LittleEndian>()?)
        } else {
            None
        };
        let color = if format.has_color {
            let red = read.read_u16::<LittleEndian>()?;
            let green = read.read_u16::<LittleEndian>()?;
            let blue = read.read_u16::<LittleEndian>()?;
            Some(Color::new(red, green, blue))
        } else {
            None
        };
        let waveform = if format.has_waveform {
            Some(Waveform::read_from(&mut read)?)
        } else {
            None
        };
        let nir = if format.has_nir {
            utils::some_or_none_if_zero(read.read_u16::<LittleEndian>()?)
        } else {
            None
        };
        let mut extra_bytes = vec![0; format.extra_bytes as usize];
        read.read_exact(&mut extra_bytes)?;
        Ok(Point {
            x: x,
            y: y,
            z: z,
            intensity: intensity,
            flags: flags,
            scan_angle: scan_angle,
            user_data: user_data,
            point_source_id: point_source_id,
            gps_time: gps_time,
            color: color,
            waveform: waveform,
            nir: nir,
            extra_bytes: extra_bytes,
        })
    }

    /// Writes a raw pont.
    ///
    /// # Examples
    ///
    /// `Write` implements `WriteRawPoint`.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::raw::Point;
    /// use las::point::Format;
    /// let mut cursor = Cursor::new(Vec::new());
    /// let point = Point::default();
    /// point.write_to(cursor, Format::default()).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W, format: Format) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        assert_eq!(format.extra_bytes as usize, self.extra_bytes.len());

        write.write_i32::<LittleEndian>(self.x)?;
        write.write_i32::<LittleEndian>(self.y)?;
        write.write_i32::<LittleEndian>(self.z)?;
        write.write_u16::<LittleEndian>(self.intensity)?;
        if format.is_extended {
            let (a, b, c) = self.flags.into();
            write.write_u8(a)?;
            write.write_u8(b)?;
            write.write_u8(c)?;
        } else {
            let (a, b) = self.flags.to_two_bytes()?;
            write.write_u8(a)?;
            write.write_u8(b)?;
        }
        if format.is_extended {
            write.write_i16::<LittleEndian>(self.scan_angle.into())?;
        } else {
            write.write_i8(self.scan_angle.into())?;
        }
        write.write_u8(self.user_data)?;
        write.write_u16::<LittleEndian>(self.point_source_id)?;
        if format.has_gps_time {
            write.write_f64::<LittleEndian>(
                self.gps_time.unwrap_or(0.0),
            )?;
        }
        if format.has_color {
            let color = self.color.unwrap_or_else(Color::default);
            write.write_u16::<LittleEndian>(color.red)?;
            write.write_u16::<LittleEndian>(color.green)?;
            write.write_u16::<LittleEndian>(color.blue)?;
        }
        if format.has_nir {
            write.write_u16::<LittleEndian>(self.nir.unwrap_or(0))?;
        }
        if format.has_waveform {
            self.waveform.unwrap_or_else(Waveform::default).write_to(
                &mut write,
            )?;
        }
        write.write_all(&self.extra_bytes)?;
        Ok(())
    }
}

impl Waveform {
    fn read_from<R: Read>(mut read: R) -> Result<Waveform> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Waveform {
            wave_packet_descriptor_index: read.read_u8()?,
            byte_offset_to_waveform_data: read.read_u64::<LittleEndian>()?,
            waveform_packet_size_in_bytes: read.read_u32::<LittleEndian>()?,
            return_point_waveform_location: read.read_f32::<LittleEndian>()?,
            x_t: read.read_f32::<LittleEndian>()?,
            y_t: read.read_f32::<LittleEndian>()?,
            z_t: read.read_f32::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, mut write: W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        write.write_u8(self.wave_packet_descriptor_index)?;
        write.write_u64::<LittleEndian>(
            self.byte_offset_to_waveform_data,
        )?;
        write.write_u32::<LittleEndian>(
            self.waveform_packet_size_in_bytes,
        )?;
        write.write_f32::<LittleEndian>(
            self.return_point_waveform_location,
        )?;
        write.write_f32::<LittleEndian>(self.x_t)?;
        write.write_f32::<LittleEndian>(self.y_t)?;
        write.write_f32::<LittleEndian>(self.z_t)?;
        Ok(())
    }
}

impl Flags {
    /// Returns the return number.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert_eq!(1, Flags::TwoByte(1, 0).return_number());
    /// assert_eq!(1, Flags::ThreeByte(1, 0, 0).return_number());
    /// ```
    pub fn return_number(&self) -> u8 {
        match *self {
            Flags::TwoByte(a, _) => a & 7,
            Flags::ThreeByte(a, _, _) => a & 15,
        }
    }

    /// Returns the number of returns.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert_eq!(1, Flags::TwoByte(8, 0).number_of_returns());
    /// assert_eq!(1, Flags::ThreeByte(16, 0, 0).number_of_returns());
    /// ```
    pub fn number_of_returns(&self) -> u8 {
        match *self {
            Flags::TwoByte(a, _) => a >> 3 & 7,
            Flags::ThreeByte(a, _, _) => a >> 4 & 15,
        }
    }

    /// Returns the scan direction from these flags.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// use las::point::ScanDirection;
    /// assert_eq!(ScanDirection::LeftToRight, Flags::TwoByte(0b01000000, 0).scan_direction());
    /// assert_eq!(ScanDirection::LeftToRight, Flags::ThreeByte(0, 0b01000000, 0).scan_direction());
    /// ```
    pub fn scan_direction(&self) -> ScanDirection {
        let n = match *self {
            Flags::TwoByte(a, _) => a,
            Flags::ThreeByte(_, b, _) => b,
        };
        if (n >> 6) & 1 == 1 {
            ScanDirection::LeftToRight
        } else {
            ScanDirection::RightToLeft
        }
    }

    /// Returns whether this point is synthetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert!(Flags::TwoByte(0, 0b00100000).is_synthetic());
    /// assert!(Flags::ThreeByte(0, 1, 0).is_synthetic());
    /// ```
    pub fn is_synthetic(&self) -> bool {
        match *self {
            Flags::TwoByte(_, b) => (b >> 5) & 1 == 1,
            Flags::ThreeByte(_, b, _) => b & 1 == 1,
        }
    }

    /// Returns whether this point is a key point.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert!(Flags::TwoByte(0, 0b01000000).is_key_point());
    /// assert!(Flags::ThreeByte(0, 2, 0).is_key_point());
    /// ```
    pub fn is_key_point(&self) -> bool {
        match *self {
            Flags::TwoByte(_, b) => (b >> 6) & 1 == 1,
            Flags::ThreeByte(_, b, _) => b & 2 == 2,
        }
    }

    /// Returns whether this point is withheld.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert!(Flags::TwoByte(0, 0b10000000).is_withheld());
    /// assert!(Flags::ThreeByte(0, 4, 0).is_withheld());
    /// ```
    pub fn is_withheld(&self) -> bool {
        match *self {
            Flags::TwoByte(_, b) => (b >> 7) & 1 == 1,
            Flags::ThreeByte(_, b, _) => b & 4 == 4,
        }
    }

    /// Returns whether this point is overlap.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert!(Flags::TwoByte(0, 12).is_overlap());
    /// assert!(Flags::ThreeByte(0, 8, 0).is_overlap());
    /// ```
    pub fn is_overlap(&self) -> bool {
        match *self {
            Flags::TwoByte(_, b) => b & 0b1111 == OVERLAP_CLASSIFICATION_CODE,
            Flags::ThreeByte(_, b, _) => b & 8 == 8,
        }
    }

    /// Returns the scanner channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert_eq!(0, Flags::TwoByte(0, 0).scanner_channel());
    /// assert_eq!(3, Flags::ThreeByte(0, 0b00110000, 0).scanner_channel());
    /// ```
    pub fn scanner_channel(&self) -> u8 {
        match *self {
            Flags::TwoByte(_, _) => 0,
            Flags::ThreeByte(_, b, _) => (b >> 4) & 3,
        }
    }

    /// Is this point the edge of a flight line?
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert!(Flags::TwoByte(128, 0).is_edge_of_flight_line());
    /// assert!(Flags::ThreeByte(0, 128, 0).is_edge_of_flight_line());
    /// ```
    pub fn is_edge_of_flight_line(&self) -> bool {
        let n = match *self {
            Flags::TwoByte(a, _) => a,
            Flags::ThreeByte(_, b, _) => b,
        };
        (n >> 7) == 1
    }

    /// Converts these flags into two bytes.
    ///
    /// If these are two byte flags, no problem. However, if these are three byte flags,
    /// information could be lost — in that case, we error.
    ///
    /// # Example
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// assert_eq!((1, 2), Flags::TwoByte(1, 2).to_two_bytes().unwrap());
    /// assert!(Flags::ThreeByte(0b00001000, 0, 0).to_two_bytes().is_err());
    /// ```
    pub fn to_two_bytes(&self) -> Result<(u8, u8)> {
        match *self {
            Flags::TwoByte(a, b) => Ok((a, b)),
            Flags::ThreeByte(_, _, c) => {
                if self.return_number() > 7 {
                    Err(Error::ReturnNumber(self.return_number(), None).into())
                } else if self.number_of_returns() > 7 {
                    Err(Error::ReturnNumber(self.number_of_returns(), None).into())
                } else if c > 31 {
                    Err(Error::Classification(c).into())
                } else if self.scanner_channel() > 0 {
                    Err(Error::ScannerChannel(self.scanner_channel()).into())
                } else {
                    let mut a = (self.number_of_returns() << 3) + self.return_number();
                    if self.scan_direction() == ScanDirection::LeftToRight {
                        a += 64;
                    }
                    if self.is_edge_of_flight_line() {
                        a += 128;
                    }
                    let mut b = if self.is_overlap() {
                        OVERLAP_CLASSIFICATION_CODE
                    } else {
                        c
                    };
                    if self.is_synthetic() {
                        b += 32;
                    }
                    if self.is_key_point() {
                        b += 64;
                    }
                    if self.is_withheld() {
                        b += 128;
                    }
                    Ok((a, b))
                }
            }
        }
    }

    /// Converts these flags to a classification.
    ///
    /// Throws an error of the classifiction is 12 (overlap points), because we don't have an
    /// overlap points class in this library. See the `las::point::Classification` documentation
    /// for more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    /// use las::point::Classification;
    /// assert_eq!(Classification::Ground, Flags::TwoByte(0, 2).to_classification().unwrap());
    /// assert_eq!(Classification::Ground, Flags::ThreeByte(0, 0, 2).to_classification().unwrap());
    /// assert!(Flags::TwoByte(0, 12).to_classification().is_err());
    /// assert!(Flags::ThreeByte(0, 0, 12).to_classification().is_err());
    /// ```
    pub fn to_classification(&self) -> Result<Classification> {
        match *self {
            Flags::TwoByte(_, b) => Classification::new(b & 0b00011111),
            Flags::ThreeByte(_, _, c) => Classification::new(c),
        }
    }

    /// Clears any overlap classes in these flags.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::raw::point::Flags;
    ///
    /// let mut flags = Flags::TwoByte(0, 12);
    /// flags.clear_overlap_class();
    /// assert_eq!(Flags::TwoByte(0, 1), flags);
    ///
    /// let mut flags = Flags::ThreeByte(0, 0, 12);
    /// flags.clear_overlap_class();
    /// assert_eq!(Flags::ThreeByte(0, 8, 1), flags);
    /// ```
    pub fn clear_overlap_class(&mut self) {
        match *self {
            Flags::TwoByte(_, ref mut b) => if *b & 0b11111 == OVERLAP_CLASSIFICATION_CODE {
                *b = (*b & 0b11100000) + u8::from(Classification::Unclassified);
            }
            Flags::ThreeByte(_, ref mut b, ref mut c) => if *c == OVERLAP_CLASSIFICATION_CODE {
                *b |= 8;
                *c = u8::from(Classification::Unclassified);
            }
        }
    }
}

impl Default for Flags {
    fn default() -> Flags {
        Flags::TwoByte(0, 0)
    }
}

impl From<Flags> for (u8, u8, u8) {
    fn from(flags: Flags) -> (u8, u8, u8) {
        match flags {
            Flags::TwoByte(_, b) => {
                let return_number = flags.return_number();
                let number_of_returns = flags.number_of_returns();
                ((number_of_returns << 4) + return_number, 0, b)
            }
            Flags::ThreeByte(a, b, c) => (a, b, c),
        }
    }
}

impl PartialEq for Flags {
    fn eq(&self, other: &Flags) -> bool {
        let (a, b, c) = (*self).into();
        let (d, e, f) = (*other).into();
        a == d && b == e && c == f
    }
}

impl Default for ScanAngle {
    fn default() -> ScanAngle {
        ScanAngle::Rank(0)
    }
}

impl From<ScanAngle> for i8 {
    fn from(scan_angle: ScanAngle) -> i8 {
        match scan_angle {
            ScanAngle::Rank(n) => n,
            ScanAngle::Scaled(_) => f32::from(scan_angle).round() as i8,
        }
    }
}

impl From<ScanAngle> for i16 {
    fn from(scan_angle: ScanAngle) -> i16 {
        match scan_angle {
            ScanAngle::Rank(n) => ScanAngle::from(f32::from(n)).into(),
            ScanAngle::Scaled(n) => n,
        }
    }
}

impl From<ScanAngle> for f32 {
    fn from(scan_angle: ScanAngle) -> f32 {
        match scan_angle {
            ScanAngle::Rank(n) => f32::from(n),
            ScanAngle::Scaled(n) => f32::from(n) * SCAN_ANGLE_SCALE_FACTOR,
        }
    }
}

impl From<f32> for ScanAngle {
    fn from(n: f32) -> ScanAngle {
        ScanAngle::Scaled((n / SCAN_ANGLE_SCALE_FACTOR) as i16)
    }
}

impl PartialEq for ScanAngle {
    fn eq(&self, other: &ScanAngle) -> bool {
        f32::from(*self) == f32::from(*other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! roundtrip {
        ($name:ident, $format:expr) => {
            mod $name {
                #[test]
                fn roundtrip() {
                    use std::io::Cursor;
                    use super::*;

                    let mut format = Format::new($format).unwrap();
                    format.extra_bytes = 1;
                    let mut point = Point::default();
                    point.extra_bytes = vec![42];
                    if format.has_color {
                        point.color = Some(Color::new(0, 0, 0));
                    }
                    if format.has_waveform {
                        point.waveform = Some(Waveform::default());
                    }
                    let mut cursor = Cursor::new(Vec::new());
                    point.write_to(&mut cursor, format).unwrap();
                    cursor.set_position(0);
                    assert_eq!(point, Point::read_from(cursor, format).unwrap());
                }
            }
        }
    }

    roundtrip!(format_0, 0);
    roundtrip!(format_1, 1);
    roundtrip!(format_2, 2);
    roundtrip!(format_3, 3);
    roundtrip!(format_4, 4);
    roundtrip!(format_5, 5);
    roundtrip!(format_6, 6);
    roundtrip!(format_7, 7);
    roundtrip!(format_8, 8);
    roundtrip!(format_9, 9);
    roundtrip!(format_10, 10);

    #[test]
    fn return_number() {
        assert_eq!((0, 0, 0), Flags::TwoByte(0, 0).into());
        assert_eq!((1, 0, 0), Flags::TwoByte(1, 0).into());
        assert_eq!((7, 0, 0), Flags::TwoByte(7, 0).into());
        assert_eq!((0u8, 0), Flags::ThreeByte(0, 0, 0).to_two_bytes().unwrap());
        assert_eq!((1u8, 0), Flags::ThreeByte(1, 0, 0).to_two_bytes().unwrap());
        assert_eq!((7u8, 0), Flags::ThreeByte(7, 0, 0).to_two_bytes().unwrap());
        assert!(Flags::ThreeByte(8, 0, 0).to_two_bytes().is_err());
        assert!(Flags::ThreeByte(15, 0, 0).to_two_bytes().is_err());
    }

    #[test]
    fn number_of_returns() {
        assert_eq!((0, 0, 0), Flags::TwoByte(0, 0).into());
        assert_eq!((16, 0, 0), Flags::TwoByte(8, 0).into());
        assert_eq!((0b01110000, 0, 0), Flags::TwoByte(0b00111000, 0).into());
        assert_eq!((0u8, 0), Flags::ThreeByte(0, 0, 0).to_two_bytes().unwrap());
        assert_eq!(
            (0b00001000u8, 0),
            Flags::ThreeByte(0b00010000, 0, 0).to_two_bytes().unwrap()
        );
        assert_eq!(
            (0b00111000u8, 0),
            Flags::ThreeByte(0b01110000, 0, 0).to_two_bytes().unwrap()
        );
        assert!(Flags::ThreeByte(0b10000000, 0, 0).to_two_bytes().is_err());
        assert!(Flags::ThreeByte(0b11110000, 0, 0).to_two_bytes().is_err());
    }

    #[test]
    fn scan_angle() {
        assert_eq!(-90i8, ScanAngle::Scaled(-15_000).into());
        assert_eq!(90i8, ScanAngle::Scaled(15_000).into());
        assert_eq!(-15_000i16, ScanAngle::Rank(-90).into());
        assert_eq!(15_000i16, ScanAngle::Rank(90).into());
    }

    #[test]
    fn is_synthetic() {
        assert!(!Flags::TwoByte(0, 0).is_synthetic());
        assert!(Flags::TwoByte(0, 0b00100000).is_synthetic());
        assert!(!Flags::ThreeByte(0, 0, 0).is_synthetic());
        assert!(Flags::ThreeByte(0, 1, 0).is_synthetic());
        assert_eq!((0, 32), Flags::ThreeByte(0, 1, 0).to_two_bytes().unwrap());
    }

    #[test]
    fn is_key_point() {
        assert!(!Flags::TwoByte(0, 0).is_key_point());
        assert!(Flags::TwoByte(0, 0b01000000).is_key_point());
        assert!(!Flags::ThreeByte(0, 0, 0).is_key_point());
        assert!(Flags::ThreeByte(0, 2, 0).is_key_point());
        assert_eq!((0, 64), Flags::ThreeByte(0, 2, 0).to_two_bytes().unwrap());
    }

    #[test]
    fn is_withheld() {
        assert!(!Flags::TwoByte(0, 0).is_withheld());
        assert!(Flags::TwoByte(0, 0b10000000).is_withheld());
        assert!(!Flags::ThreeByte(0, 0, 0).is_withheld());
        assert!(Flags::ThreeByte(0, 4, 0).is_withheld());
        assert_eq!((0, 128), Flags::ThreeByte(0, 4, 0).to_two_bytes().unwrap());
    }

    #[test]
    fn is_overlap() {
        assert!(!Flags::TwoByte(0, 0).is_overlap());
        assert!(Flags::TwoByte(0, OVERLAP_CLASSIFICATION_CODE).is_overlap());
        assert!(!Flags::ThreeByte(0, 0, 0).is_overlap());
        assert!(Flags::ThreeByte(0, 8, 0).is_overlap());
        assert_eq!(
            (0, OVERLAP_CLASSIFICATION_CODE),
            Flags::ThreeByte(0, 8, 0).to_two_bytes().unwrap()
        );
    }

    #[test]
    fn scanner_channel() {
        assert_eq!(0, Flags::TwoByte(0, 0).scanner_channel());
        assert_eq!(0, Flags::ThreeByte(0, 0, 0).scanner_channel());
        assert_eq!(1, Flags::ThreeByte(0, 0b00010000, 0).scanner_channel());
        assert_eq!(3, Flags::ThreeByte(0, 0b00110000, 0).scanner_channel());
    }

    #[test]
    fn scan_direction() {
        assert_eq!(
            ScanDirection::RightToLeft,
            Flags::TwoByte(0, 0).scan_direction()
        );
        assert_eq!(
            ScanDirection::LeftToRight,
            Flags::TwoByte(0b01000000, 0).scan_direction()
        );
        assert_eq!(
            ScanDirection::RightToLeft,
            Flags::ThreeByte(0, 0, 0).scan_direction()
        );
        assert_eq!(
            ScanDirection::LeftToRight,
            Flags::ThreeByte(0, 0b01000000, 0).scan_direction()
        );
    }

    #[test]
    fn is_edge_of_flight_line() {
        assert!(!Flags::TwoByte(0, 0).is_edge_of_flight_line());
        assert!(Flags::TwoByte(0b10000000, 0).is_edge_of_flight_line());
        assert!(!Flags::ThreeByte(0, 0, 0).is_edge_of_flight_line());
        assert!(Flags::ThreeByte(0, 0b10000000, 0).is_edge_of_flight_line());
        assert_eq!(
            (128, 0),
            Flags::ThreeByte(0, 128, 0).to_two_bytes().unwrap()
        );
    }

    #[test]
    fn classification() {
        assert_eq!((0, 1), Flags::ThreeByte(0, 0, 1).to_two_bytes().unwrap());
        assert!(Flags::ThreeByte(0, 0, 32).to_two_bytes().is_err());
    }
}
