use error::Error;
use laz::las::laszip::{LazVlr, LASZIP_DESCRIPTION, LASZIP_RECORD_ID, LASZIP_USER_ID};
use reader::{read_point_from, PointReader};
use std::fmt::Debug;
/// Module with functions and structs specific to brigde the las crate and laz crate to allow
/// writing & reading LAZ data
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use writer::{write_header_and_vlrs_to, write_point_to, PointWriter};
use {Header, Point, Result, Vlr};

fn is_laszip_vlr(vlr: &Vlr) -> bool {
    if &vlr.user_id == LASZIP_USER_ID && vlr.record_id == LASZIP_RECORD_ID {
        true
    } else {
        false
    }
}

fn create_laszip_vlr(laszip_vlr: &LazVlr) -> std::io::Result<Vlr> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    laszip_vlr.write_to(&mut cursor)?;
    Ok(Vlr {
        user_id: LASZIP_USER_ID.to_owned(),
        record_id: LASZIP_RECORD_ID,
        description: LASZIP_DESCRIPTION.to_owned(),
        data: cursor.into_inner(),
    })
}

/// struct that knows how to decompress LAZ
///
/// Decompression is done in 2 steps:
/// 1) call the decompressor that reads & decompress the next point
/// and put its data in an in-memory buffer
/// 2) read the buffer to get the decompress point
pub(crate) struct CompressedPointReader<'a, R: Read + Seek> {
    /// decompressor that does the actual job
    decompressor: laz::las::laszip::LasZipDecompressor<'a, R>,
    header: Header,
    /// in-memory buffer where the decompressor writes decompression result
    decompressor_output: Cursor<Vec<u8>>,
    last_point_idx: u64,
}

impl<'a, R: Read + Seek> CompressedPointReader<'a, R> {
    pub(crate) fn new(source: R, header: Header) -> Result<Self> {
        let laszip_vlr = match header.vlrs().iter().find(|vlr| is_laszip_vlr(*vlr)) {
            None => return Err(Error::LasZipVlrNotFound),
            Some(ref vlr) => laz::las::laszip::LazVlr::from_buffer(&vlr.data)?,
        };
        let decompressor_output = Cursor::new(vec![0u8; header.point_format().len() as usize]);

        Ok(Self {
            decompressor: laz::las::laszip::LasZipDecompressor::new(source, laszip_vlr)?,
            header,
            decompressor_output,
            last_point_idx: 0,
        })
    }
}

impl<'a, R: Read + Seek> Debug for CompressedPointReader<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CompressedPointReader(num_read: {}, header: {:?})",
            self.last_point_idx, self.header
        )
    }
}

impl<'a, R: Read + Seek> PointReader for CompressedPointReader<'a, R> {
    fn read_next(&mut self) -> Option<Result<Point>> {
        if self.last_point_idx < self.header.number_of_points() {
            self.last_point_idx += 1;
            self.decompressor
                .decompress_one(&mut self.decompressor_output.get_mut())
                .unwrap();
            if let Err(e) = self.decompressor_output.seek(SeekFrom::Start(0)) {
                Some(Err(e.into()))
            } else {
                Some(read_point_from(&mut self.decompressor_output, &self.header))
            }
        } else {
            None
        }
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        self.last_point_idx = position - 1;
        self.decompressor.seek(position)?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}

/// struct that knows how to write LAZ
///
/// Writing a point compressed is done in 2 steps
/// 1) write the point to a in-memory buffer
/// 2) call the laz compressor on this buffer
pub(crate) struct CompressedPointWriter<'a, W: Write + Seek> {
    header: Header,
    /// buffer used to write the uncompressed point
    compressor_input: Cursor<Vec<u8>>,
    /// The compressor that actually does the job of compressing the data
    compressor: laz::las::laszip::LasZipCompressor<'a, W>,
}

impl<'a, W: Write + Seek> CompressedPointWriter<'a, W> {
    pub(crate) fn new(mut dest: W, mut header: Header) -> Result<Self> {
        if header.point_format().is_extended {
            panic!("Writing Extended point data is not supported");
        }

        let mut laz_items = laz::las::laszip::LazItemRecordBuilder::new();
        laz_items.add_item(laz::las::laszip::LazItemType::Point10);

        if header.point_format().has_gps_time {
            laz_items.add_item(laz::las::laszip::LazItemType::GpsTime);
        }

        if header.point_format().has_color {
            laz_items.add_item(laz::las::laszip::LazItemType::RGB12);
        }

        if header.point_format().extra_bytes > 0 {
            laz_items.add_item(laz::las::laszip::LazItemType::Byte(
                header.point_format().extra_bytes,
            ));
        }

        let laz_vlr = LazVlr::from_laz_items(laz_items.build());
        // Clear any existing laszip vlr as they might not be correct
        header.vlrs_mut().retain(|vlr| !is_laszip_vlr(vlr));
        header.vlrs_mut().push(create_laszip_vlr(&laz_vlr)?);

        write_header_and_vlrs_to(&mut dest, &header)?;

        let compressor_input = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let compressor = laz::las::laszip::LasZipCompressor::from_laz_vlr(dest, laz_vlr)?;

        Ok(Self {
            header,
            compressor_input,
            compressor,
        })
    }
}

impl<'a, W: Write + Seek> PointWriter<W> for CompressedPointWriter<'a, W> {
    fn write_next(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        self.compressor_input.seek(SeekFrom::Start(0))?;
        write_point_to(&mut self.compressor_input, point, &self.header)?;
        self.compressor
            .compress_one(self.compressor_input.get_ref())?;
        Ok(())
    }

    fn into_inner(self: Box<Self>) -> W {
        self.compressor.into_stream()
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

impl<'a, W: Write + Seek> Debug for CompressedPointWriter<'a, W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompressedPointWriter(header: {:?})", self.header)
    }
}
