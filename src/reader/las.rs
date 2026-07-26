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
    fn fill_into_bytes(&mut self, n: u64, out: &mut Vec<u8>, record_len: usize) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);
        let n_usize = usize::try_from(n)?;
        // The header's `number_of_point_records` is untrusted and may name far
        // more points than the stream carries. Honouring it verbatim lets a
        // sub-kibibyte file force a multi-gibibyte allocation (or a `capacity
        // overflow` panic once `n_usize * record_len` saturates `usize`) before
        // a single point byte is read. Guard the multiply against `usize`
        // overflow, and refuse to reserve more bytes than the stream actually
        // holds — `read_exact` below still reports genuine short reads, so this
        // only rejects allocations a valid file could never satisfy.
        let alloc_bytes = n_usize.checked_mul(record_len).ok_or_else(|| {
            crate::Error::PointRecordLengthOverflow {
                n,
                record_len: u16::try_from(record_len).unwrap_or(u16::MAX),
            }
        })?;
        let cur = self.read.stream_position()?;
        let stream_end = self.read.seek(SeekFrom::End(0))?;
        let _ = self.read.seek(SeekFrom::Start(cur));
        let remaining = stream_end.saturating_sub(cur);
        if u64::try_from(alloc_bytes).unwrap_or(u64::MAX) > remaining {
            return Err(crate::Error::PointRecordLengthOverflow {
                n,
                record_len: u16::try_from(record_len).unwrap_or(u16::MAX),
            });
        }
        out.resize(alloc_bytes, 0u8);
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
