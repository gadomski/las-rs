use crate::error::Error;
use crate::reader::{read_point_from, PointReader};
use crate::writer::{write_header_and_vlrs_to, write_point_to, PointWriter};
use crate::{Header, Point, Result, Vlr};
use laz::las::laszip::LazVlr;
use std::fmt::Debug;
/// Module with functions and structs specific to brigde the las crate and laz crate to allow
/// writing & reading LAZ data
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

fn is_laszip_vlr(vlr: &Vlr) -> bool {
    vlr.user_id == LazVlr::USER_ID && vlr.record_id == LazVlr::RECORD_ID
}

fn create_laszip_vlr(laszip_vlr: &LazVlr) -> std::io::Result<Vlr> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    laszip_vlr.write_to(&mut cursor)?;
    Ok(Vlr {
        user_id: LazVlr::USER_ID.to_owned(),
        record_id: LazVlr::RECORD_ID,
        description: LazVlr::DESCRIPTION.to_owned(),
        data: cursor.into_inner(),
    })
}

/// struct that knows how to decompress LAZ
///
/// Decompression is done in 2 steps:
/// 1) call the decompressor that reads & decompress the next point
/// and put its data in an in-memory buffer
/// 2) read the buffer to get the decompress point
pub(crate) struct CompressedPointReader<Decompressor> {
    /// decompressor that does the actual job
    decompressor: Decompressor,
    header: Header,
    /// in-memory buffer where the decompressor writes decompression result
    decompressor_output: Cursor<Vec<u8>>,
    last_point_idx: u64,
}

#[cfg(not(feature = "laz-parallel"))]
impl<'a, R: Read + Seek + Send> CompressedPointReader<laz::LasZipDecompressor<'a, R>> {
    pub(crate) fn new(source: R, header: Header) -> Result<Self> {
        let laszip_vlr = match header.vlrs().iter().find(|vlr| is_laszip_vlr(*vlr)) {
            None => return Err(Error::LasZipVlrNotFound),
            Some(vlr) => LazVlr::from_buffer(&vlr.data)?,
        };
        let decompressor_output = Cursor::new(vec![0u8; header.point_format().len() as usize]);

        Ok(Self {
            decompressor: laz::LasZipDecompressor::new(source, laszip_vlr)?,
            header,
            decompressor_output,
            last_point_idx: 0,
        })
    }
}

#[cfg(feature = "laz-parallel")]
impl<R: Read + Seek + Send> CompressedPointReader<laz::ParLasZipDecompressor<R>> {
    pub(crate) fn new(source: R, header: Header) -> Result<Self> {
        let laszip_vlr = match header.vlrs().iter().find(|vlr| is_laszip_vlr(*vlr)) {
            None => return Err(Error::LasZipVlrNotFound),
            Some(vlr) => LazVlr::from_buffer(&vlr.data)?,
        };
        let decompressor_output = Cursor::new(vec![0u8; header.point_format().len() as usize]);

        Ok(Self {
            decompressor: laz::ParLasZipDecompressor::new(source, laszip_vlr)?,
            header,
            decompressor_output,
            last_point_idx: 0,
        })
    }
}

impl<Decompressor> Debug for CompressedPointReader<Decompressor> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CompressedPointReader(num_read: {}, header: {:?})",
            self.last_point_idx, self.header
        )
    }
}

impl<Decompressor> PointReader for CompressedPointReader<Decompressor>
where
    Decompressor: laz::LazDecompressor + Send,
{
    fn read_next(&mut self) -> Option<Result<Point>> {
        if self.last_point_idx < self.header.number_of_points() {
            self.last_point_idx += 1;
            let res = self
                .decompressor
                .decompress_one(&mut self.decompressor_output.get_mut());
            if let Err(e) = res {
                Some(Err(e.into()))
            } else {
                self.decompressor_output.set_position(0);
                Some(read_point_from(&mut self.decompressor_output, &self.header))
            }
        } else {
            None
        }
    }

    fn read_into_vec(&mut self, points: &mut Vec<Point>, n: u64) -> Result<u64> {
        let points_left = self.header().number_of_points() - self.last_point_idx;
        let num_points_to_read = points_left.min(n);

        self.decompressor_output.get_mut().resize(
            num_points_to_read as usize * self.header.point_format().len() as usize,
            0u8,
        );
        self.decompressor
            .decompress_many(self.decompressor_output.get_mut())?;
        self.decompressor_output.set_position(0);
        points.reserve(num_points_to_read as usize);

        for _ in 0..num_points_to_read {
            let point = read_point_from(&mut self.decompressor_output, &self.header)?;
            self.last_point_idx += 1;
            points.push(point);
        }
        Ok(num_points_to_read)
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        self.last_point_idx = position;
        self.decompressor.seek(position)?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}

fn laz_vlr_from_point_format(point_format: &crate::point::Format) -> LazVlr {
    let mut laz_items = laz::las::laszip::LazItemRecordBuilder::new();
    if !point_format.is_extended {
        laz_items.add_item(laz::LazItemType::Point10);

        if point_format.has_gps_time {
            laz_items.add_item(laz::LazItemType::GpsTime);
        }

        if point_format.has_color {
            laz_items.add_item(laz::LazItemType::RGB12);
        }

        if point_format.extra_bytes > 0 {
            laz_items.add_item(laz::LazItemType::Byte(point_format.extra_bytes));
        }
    } else {
        laz_items.add_item(laz::LazItemType::Point14);

        if point_format.has_color {
            // Point format 7 & 8 both have RGB
            if point_format.has_nir {
                laz_items.add_item(laz::LazItemType::RGBNIR14);
            } else {
                laz_items.add_item(laz::LazItemType::RGB14);
            }
        }
        if point_format.extra_bytes > 0 {
            laz_items.add_item(laz::LazItemType::Byte14(point_format.extra_bytes));
        }
    }
    laz::LazVlr::from_laz_items(laz_items.build())
}

/// struct that knows how to write LAZ
///
/// Writing a point compressed is done in 2 steps
/// 1) write the point to a in-memory buffer
/// 2) call the laz compressor on this buffer
pub(crate) struct CompressedPointWriter<'a, W: Write + Seek + Send> {
    header: Header,
    /// buffer used to write the uncompressed point
    compressor_input: Cursor<Vec<u8>>,
    /// The compressor that actually does the job of compressing the data
    compressor: laz::las::laszip::LasZipCompressor<'a, W>,
}

impl<'a, W: Write + Seek + Send> CompressedPointWriter<'a, W> {
    pub(crate) fn new(mut dest: W, mut header: Header) -> Result<Self> {
        let laz_vlr = laz_vlr_from_point_format(header.point_format());
        // Clear any existing laszip vlr as they might not be correct
        header.vlrs_mut().retain(|vlr| !is_laszip_vlr(vlr));
        header.vlrs_mut().push(create_laszip_vlr(&laz_vlr)?);

        write_header_and_vlrs_to(&mut dest, &header)?;

        let compressor_input = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let compressor = laz::las::laszip::LasZipCompressor::new(dest, laz_vlr)?;

        Ok(Self {
            header,
            compressor_input,
            compressor,
        })
    }
}

impl<'a, W: Write + Seek + Send> PointWriter<W> for CompressedPointWriter<'a, W> {
    fn write_next(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        self.compressor_input.seek(SeekFrom::Start(0))?;
        write_point_to(&mut self.compressor_input, point, &self.header)?;
        self.compressor
            .compress_one(self.compressor_input.get_ref())?;
        Ok(())
    }

    fn into_inner(self: Box<Self>) -> W {
        self.compressor.into_inner()
    }

    fn get_mut(&mut self) -> &mut W {
        self.compressor.get_mut()
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn done(&mut self) -> Result<()> {
        self.compressor.done()?;
        Ok(())
    }
}

impl<'a, W: Write + Seek + Send> Debug for CompressedPointWriter<'a, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompressedPointWriter(header: {:?})", self.header)
    }
}
