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
    fn fill_into_bytes(&mut self, n: u64, out: &mut Vec<u8>, record_len: usize) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);
        let n_usize = usize::try_from(n)?;
        // Guard the byte count against `usize` overflow before `resize`. A
        // crafted header can make `number_of_point_records * record_len`
        // saturate `usize`, which `Vec::resize` turns into a `capacity
        // overflow` panic; `decompress_many` already reports truncated input,
        // so we only reject what no valid stream could back.
        let alloc_bytes = n_usize.checked_mul(record_len).ok_or_else(|| {
            crate::Error::PointRecordLengthOverflow {
                n,
                record_len: u16::try_from(record_len).unwrap_or(u16::MAX),
            }
        })?;
        out.resize(alloc_bytes, 0u8);
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
