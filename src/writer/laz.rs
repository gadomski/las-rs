use super::WritePoint;
use crate::{Error, Header, Point, Result};
use ::laz::LasZipCompressor;
use std::io::{Cursor, Seek, Write};

pub(crate) struct PointWriter<'a, W: Write + Seek + Send> {
    compressor: LasZipCompressor<'a, W>,
    buffer: Cursor<Vec<u8>>,
    header: Header,
}

impl<'a, W: Write + Seek + Send> PointWriter<'a, W> {
    pub(crate) fn new(write: W, header: Header) -> Result<PointWriter<'a, W>> {
        let buffer = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let vlr = header.laz_vlr().ok_or(Error::LasZipVlrNotFound)?;
        let compressor = LasZipCompressor::new(write, vlr)?;

        Ok(Self {
            header,
            buffer,
            compressor,
        })
    }
}

impl<W: Write + Seek + Send> WritePoint<W> for PointWriter<'_, W> {
    fn write_point(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        self.buffer.set_position(0);
        point
            .into_raw(self.header.transforms())
            .and_then(|raw_point| {
                raw_point.write_to(&mut self.buffer, self.header.point_format())
            })?;
        self.compressor
            .compress_one(self.buffer.get_ref())
            .map_err(Error::from)
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

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }

    fn done(&mut self) -> Result<()> {
        self.compressor.done()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Builder, Point, Reader, Vlr, Writer};
    use std::io::Cursor;

    #[test]
    fn evlr() {
        let mut vlr = Vlr::default();
        vlr.user_id = "@gadomski".to_string();
        vlr.record_id = 42;
        vlr.description = "A great vlr".to_string();
        vlr.data = b"some data".to_vec();
        let mut builder = Builder::default();
        builder.version.minor = 4;
        builder.point_format.is_compressed = true;
        builder.evlrs.push(vlr);
        let header = builder.into_header().unwrap();
        let cursor = Cursor::new(Vec::new());
        let mut writer = Writer::new(cursor, header).unwrap();
        for i in 0..5 {
            let mut point = Point::default();
            point.return_number = i;
            writer.write_point(point).unwrap();
        }
        let cursor = writer.into_inner().unwrap();
        let reader = Reader::new(cursor).unwrap();
        let evlr = &reader.header().evlrs()[0];
        assert_eq!(evlr.user_id, "@gadomski");
        assert_eq!(evlr.record_id, 42);
        assert_eq!(evlr.description, "A great vlr");
        assert_eq!(evlr.data, b"some data");
    }
}
