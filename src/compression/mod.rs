/// Module with functions and structs specific to brigde the las crate and laz crate to allow
/// writing & reading LAZ data

use std::io::{Write, Seek, Cursor, Read, SeekFrom};
use {Header, Point, Result, Vlr};
use writer::{PointWriter, write_point_to, write_header_and_vlrs_to};
use reader::{PointReader, read_point_from};
use laz::las::laszip::{LazVlr, LASZIP_DESCRIPTION, LASZIP_RECORD_ID, LASZIP_USER_ID};
use std::fmt::Debug;
use error::Error;


pub(crate) fn create_laszip_vlr(laszip_vlr: &LazVlr) -> std::io::Result<Vlr> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    laszip_vlr.write_to(&mut cursor)?;
    Ok(Vlr{
        user_id: LASZIP_USER_ID.to_owned(),
        record_id: LASZIP_RECORD_ID,
        description: LASZIP_DESCRIPTION.to_owned(),
        data: cursor.into_inner()
    })
}

pub(crate) fn extract_laszip_vlr(header: &mut Header) -> Option<Vlr> {
    let mut index = None;
    for (i, vlr) in header.vlrs().iter().enumerate() {
        if &vlr.user_id == LASZIP_USER_ID && vlr.record_id == LASZIP_RECORD_ID {
            index = Some(i);
        }
    }
    match index {
        Some(i) => Some(header.vlrs_mut().remove(i)),
        None => None
    }
}

/// struct that knows how to decompress LAZ
///
/// Decompression is done in 2 step:
/// 1) call the decompressor the read & decompress the next point
/// and put its data in a in-memory buffer
/// 2) read the buffer to get the decompress point
pub(crate) struct CompressedPointReader<R: Read +Seek> {
    /// decompressor that does the actual job
    decompressor: laz::las::laszip::LasZipDecompressor<R>,
    header: Header,
    /// in-memory buffer where the decompressor writes deompression result
    decomp_out: Cursor<Vec<u8>>,
    last_point_idx: u64,
}

impl<R: Read + Seek> CompressedPointReader<R> {
    pub(crate) fn new(source: R, mut header: Header) -> Result<Self> {
        let laszip_vlr = match extract_laszip_vlr(&mut header) {
            None => return Err(Error::LasZipVlrNotFound),
            Some(vlr) => laz::las::laszip::LazVlr::from_buffer(&vlr.data)?
        };
        let decomp_out = Cursor::new(vec![0u8; header.point_format().len() as usize]);

        Ok(Self {
            decompressor: laz::las::laszip::LasZipDecompressor::new(source, laszip_vlr)?,
            header,
            decomp_out,
            last_point_idx: 0,
        })
    }
}

impl<R: Read + Seek> Debug for CompressedPointReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompressedPointReader(num_read: {}, header: {:?})", self.last_point_idx, self.header)
    }
}

impl<R: Read + Seek> PointReader for CompressedPointReader<R> {
    fn read_next(&mut self) -> Option<Result<Point>> {
        if self.last_point_idx < self.header.number_of_points() {
            self.last_point_idx += 1;
            self.decompressor.decompress_one(&mut self.decomp_out.get_mut()).unwrap();
            if let Err(e) = self.decomp_out.seek(SeekFrom::Start(0)) {
                Some(Err(e.into()))
            } else {
                Some(read_point_from(&mut self.decomp_out, &self.header))
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
pub(crate) struct CompressedPointWriter<W: Write + Seek> {
    header: Header,
    /// buffer used to write the un compressed point
    compression_in: Cursor<Vec<u8>>,
    /// The compressor that actually doest the job of compressing the data
    compressor: laz::las::laszip::LasZipCompressor<W>
}


impl<W: Write + Seek> CompressedPointWriter<W> {
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
            laz_items.add_item(laz::las::laszip::LazItemType::Byte(header.point_format().extra_bytes));
        }

        let laz_vlr = LazVlr::from_laz_items(laz_items.build());
        header.vlrs_mut().push(create_laszip_vlr(&laz_vlr)?);

        write_header_and_vlrs_to(&mut dest, &header)?;

        let compression_in = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let compressor = laz::las::laszip::LasZipCompressor::from_laz_vlr(dest, laz_vlr)?;

        Ok(Self {
            header,
            compression_in,
            compressor
        })
    }
}

impl<W: Write + Seek> PointWriter<W> for CompressedPointWriter<W> {
    fn write_next(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        self.compression_in.seek(SeekFrom::Start(0))?;
        write_point_to(&mut self.compression_in, point, &self.header)?;
        self.compressor.compress_one(self.compression_in.get_ref())?;
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

impl<W: Write + Seek> Debug for CompressedPointWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "CompressedPointWriter(header: {:?})", self.header)
    }
}