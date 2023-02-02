//! Point::read_from_async
use crate::point::Format;
use crate::raw::point::{Flags, ScanAngle, Waveform};
use crate::raw::Point;
use crate::Color;
use crate::Result;
use byteorder_async::ReaderToByteOrder;
use futures::io::AsyncRead;

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
    /// let point = Point::read_from(file, &Format::new(1).unwrap()).unwrap();
    /// ```
    #[allow(clippy::field_reassign_with_default)]
    pub async fn read_from_async<R: AsyncRead + Unpin>(
        mut read: R,
        format: &Format,
    ) -> Result<Point> {
        use crate::utils;
        use byteorder_async::LittleEndian;

        let mut read = read.byte_order();

        let mut point = Point::default();
        point.x = read.read_i32::<LittleEndian>().await?;
        point.y = read.read_i32::<LittleEndian>().await?;
        point.z = read.read_i32::<LittleEndian>().await?;
        point.intensity = read.read_u16::<LittleEndian>().await?;
        point.flags = if format.is_extended {
            Flags::ThreeByte(
                read.read_u8().await?,
                read.read_u8().await?,
                read.read_u8().await?,
            )
        } else {
            Flags::TwoByte(read.read_u8().await?, read.read_u8().await?)
        };
        if format.is_extended {
            point.user_data = read.read_u8().await?;
            point.scan_angle = ScanAngle::Scaled(read.read_i16::<LittleEndian>().await?);
        } else {
            point.scan_angle = ScanAngle::Rank(read.read_i8().await?);
            point.user_data = read.read_u8().await?;
        };
        point.point_source_id = read.read_u16::<LittleEndian>().await?;
        point.gps_time = if format.has_gps_time {
            utils::some_or_none_if_zero(read.read_f64::<LittleEndian>().await?)
        } else {
            None
        };
        point.color = if format.has_color {
            let red = read.read_u16::<LittleEndian>().await?;
            let green = read.read_u16::<LittleEndian>().await?;
            let blue = read.read_u16::<LittleEndian>().await?;
            Some(Color::new(red, green, blue))
        } else {
            None
        };
        point.waveform = if format.has_waveform {
            Some(Waveform::read_from_async(read.get_mut()).await?)
        } else {
            None
        };
        point.nir = if format.has_nir {
            utils::some_or_none_if_zero(read.read_u16::<LittleEndian>().await?)
        } else {
            None
        };
        point.extra_bytes.resize(format.extra_bytes as usize, 0);
        read.read_exact(&mut point.extra_bytes).await?;
        Ok(point)
    }
}

impl Waveform {
    async fn read_from_async<R: AsyncRead + Unpin>(mut read: R) -> Result<Waveform> {
        todo!()
    }
}
