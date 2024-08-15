use super::ReadPoints;
use crate::{raw, Error, Header, Point, Result, Vlr};
use laz::{LazDecompressor, LazVlr};
use std::io::{Cursor, Read, Seek};

pub(crate) struct PointReader<D: LazDecompressor> {
    buffer: Cursor<Vec<u8>>,
    decompressor: D,
    header: Header,
    index: u64,
}

#[cfg(feature = "laz-parallel")]
impl<R: Read + Seek> PointReader<laz::ParLasZipDecompressor<R>> {
    pub(crate) fn new(
        read: R,
        header: Header,
    ) -> Result<PointReader<laz::ParLasZipDecompressor<R>>> {
        let (vlr, buffer) = vlr_and_buffer(&header)?;
        let decompressor = laz::ParLasZipDecompressor::new(read, vlr)?;
        Ok(PointReader {
            decompressor,
            header,
            buffer,
            index: 0,
        })
    }
}

#[cfg(not(feature = "laz-parallel"))]
impl<R: Read + Seek + Send> PointReader<laz::LasZipDecompressor<'_, R>> {
    pub(crate) fn new(
        read: R,
        header: Header,
    ) -> Result<PointReader<laz::LasZipDecompressor<'static, R>>> {
        let (vlr, buffer) = vlr_and_buffer(&header)?;
        let decompressor = laz::LasZipDecompressor::new(read, vlr)?;
        Ok(PointReader {
            decompressor,
            header,
            buffer,
            index: 0,
        })
    }
}

impl<D> ReadPoints for PointReader<D>
where
    D: LazDecompressor + Send,
{
    fn read_point(&mut self) -> Result<Option<Point>> {
        if self.index < self.header.number_of_points() {
            self.index += 1;
            self.decompressor.decompress_one(self.buffer.get_mut())?;
            self.buffer.set_position(0);
            raw::Point::read_from(&mut self.buffer, self.header.point_format())
                .map(|raw_point| Point::new(raw_point, self.header.transforms()))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_points(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);

        let resize = usize::try_from(n * u64::from(self.header.point_format().len()))?;
        self.buffer.get_mut().resize(resize, 0u8);
        self.decompressor.decompress_many(self.buffer.get_mut())?;
        self.buffer.set_position(0);
        if let Ok(n) = usize::try_from(n) {
            points.reserve(n);
        }

        for _ in 0..n {
            let point = raw::Point::read_from(&mut self.buffer, self.header.point_format())
                .map(|raw_point| Point::new(raw_point, self.header.transforms()))?;
            self.index += 1;
            points.push(point);
        }
        Ok(n)
    }

    fn seek(&mut self, index: u64) -> Result<()> {
        self.index = index;
        self.decompressor.seek(index)?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}

fn is_laszip_vlr(vlr: &Vlr) -> bool {
    vlr.user_id == LazVlr::USER_ID && vlr.record_id == LazVlr::RECORD_ID
}

fn vlr_and_buffer(header: &Header) -> Result<(LazVlr, Cursor<Vec<u8>>)> {
    let vlr = match header.vlrs().iter().find(|vlr| is_laszip_vlr(vlr)) {
        None => return Err(Error::LasZipVlrNotFound),
        Some(vlr) => LazVlr::from_buffer(&vlr.data)?,
    };
    let buffer = Cursor::new(vec![0u8; header.point_format().len().into()]);
    Ok((vlr, buffer))
}
