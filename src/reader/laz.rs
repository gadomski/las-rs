use super::ReadPoints;
use crate::{Header, Result};
use laz::LazDecompressor;
use std::io::{Read, Seek};

pub(crate) struct PointReader<D: LazDecompressor> {
    decompressor: D,
    header: Header,
    index: u64,
}

#[cfg(feature = "laz-parallel")]
impl<R: Read + Seek> PointReader<laz::ParLasZipDecompressor<R>> {
    pub(crate) fn new_parallel(
        read: R,
        header: Header,
    ) -> Result<PointReader<laz::ParLasZipDecompressor<R>>> {
        let decompressor = laz::ParLasZipDecompressor::new(read, header.laz_vlr()?)?;
        Ok(PointReader {
            decompressor,
            header,
            index: 0,
        })
    }
}

impl<R: Read + Seek + Send + Sync> PointReader<laz::LasZipDecompressor<'_, R>> {
    pub(crate) fn new(
        read: R,
        header: Header,
    ) -> Result<PointReader<laz::LasZipDecompressor<'static, R>>> {
        let decompressor = laz::LasZipDecompressor::new(read, header.laz_vlr()?)?;
        Ok(PointReader {
            decompressor,
            header,
            index: 0,
        })
    }
}

impl<D> ReadPoints for PointReader<D>
where
    D: LazDecompressor + Send,
{
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
        self.decompressor.decompress_many(out)?;
        self.index += n;
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
