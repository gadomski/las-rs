use super::WritePoint;
use crate::{Error, Header, Point, Result};
use ::laz::{LasZipCompressor, LazCompressor, LazCompressorWithInner};
#[cfg(feature = "laz-parallel")]
use laz::ParLasZipCompressor;
use std::io::{Cursor, Seek, Write};

pub(crate) struct PointWriter<C> {
    compressor: C,
    // Buffer with raw bytes of the points to be compressed
    buffer: Cursor<Vec<u8>>,
    header: Header,
}

impl<'a, W: Write + Seek + Send + Sync> PointWriter<LasZipCompressor<'a, W>> {
    pub(crate) fn new(write: W, header: Header) -> Result<PointWriter<LasZipCompressor<'a, W>>> {
        let buffer = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let vlr = header.laz_vlr()?;
        let compressor = LasZipCompressor::new(write, vlr)?;

        Ok(Self {
            header,
            buffer,
            compressor,
        })
    }
}

#[cfg(feature = "laz-parallel")]
impl<W: Write + Seek + Send + Sync> PointWriter<ParLasZipCompressor<W>> {
    pub(crate) fn new_parallel(
        write: W,
        header: Header,
    ) -> Result<PointWriter<ParLasZipCompressor<W>>> {
        let buffer = Cursor::new(vec![0u8; header.point_format().len() as usize]);
        let vlr = header.laz_vlr()?;
        let compressor = ParLasZipCompressor::new(write, vlr)?;

        Ok(Self {
            header,
            buffer,
            compressor,
        })
    }
}

impl<W, C> WritePoint<W> for PointWriter<C>
where
    C: LazCompressor + LazCompressorWithInner<W> + Send + Sync,
    W: Write + Seek + Send + Sync,
{
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

    fn write_points(&mut self, points: &[Point]) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }

        let current_cap = self.buffer.get_ref().capacity();
        let necessary_cap = self.header.point_format().len() as usize * points.len();
        if necessary_cap > current_cap {
            self.buffer.get_mut().reserve(necessary_cap - current_cap);
        }
        self.buffer.set_position(0);

        for point in points.iter().cloned() {
            self.header.add_point(&point);
            let raw_point = point.into_raw(self.header.transforms())?;
            raw_point.write_to(&mut self.buffer, self.header.point_format())?;
        }

        let len = self.buffer.position() as usize;
        let buffer = &self.buffer.get_ref()[..len];
        self.compressor.compress_many(buffer)?;
        Ok(())
    }

    fn into_inner(self: Box<Self>) -> W {
        self.compressor.into_inner()
    }

    fn get_mut(&mut self) -> &mut W {
        self.compressor.inner_mut()
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
        let vlr = Vlr {
            user_id: "@gadomski".to_string(),
            record_id: 42,
            description: "A great vlr".to_string(),
            data: b"some data".to_vec(),
        };
        let mut builder = Builder::default();
        builder.version.minor = 4;
        builder.point_format.is_compressed = true;
        builder.evlrs.push(vlr);
        let header = builder.into_header().unwrap();
        let cursor = Cursor::new(Vec::new());
        let mut writer = Writer::new(cursor, header).unwrap();
        for i in 0..5 {
            let point = Point {
                return_number: i,
                ..Default::default()
            };
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
