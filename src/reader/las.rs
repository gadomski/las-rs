use super::ReadPoints;
use crate::{Header, Result};
use std::io::{Read, Seek, SeekFrom};

pub(crate) struct PointReader<R: Read + Seek> {
    read: R,
    header: Header,
    index: u64,
    start: u64,
}

impl<R: Read + Seek> PointReader<R> {
    pub(crate) fn new(mut read: R, header: Header) -> Result<PointReader<R>> {
        Ok(PointReader {
            start: read.stream_position()?,
            read,
            header,
            index: 0,
        })
    }
}

impl<R: Read + Seek> ReadPoints for PointReader<R> {
    fn fill_into_bytes(
        &mut self,
        n: u64,
        out: &mut Vec<u8>,
        record_len: usize,
    ) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);
        let n_usize = usize::try_from(n)?;
        out.resize(n_usize * record_len, 0u8);
        self.read.read_exact(out)?;
        self.index += n;
        Ok(n)
    }

    fn seek(&mut self, index: u64) -> Result<()> {
        self.index = index;
        let _ = self.read.seek(SeekFrom::Start(
            self.start + index * u64::from(self.header.point_format().len()),
        ))?;
        Ok(())
    }

    fn header(&self) -> &Header {
        &self.header
    }
}
