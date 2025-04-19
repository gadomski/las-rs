//! Copc Entry Reader

use crate::{copc, raw, Header, Point, Result};
use laz::record::{LayeredPointRecordDecompressor, RecordDecompressor};
use std::io::{Cursor, Read, Seek, SeekFrom};

#[allow(missing_debug_implementations)]
/// Entry Reader can read whole entrys of copc laz files
pub struct CopcEntryReader<'a, R: Read + Seek> {
    decompressor: LayeredPointRecordDecompressor<'a, R>,
    buffer: Cursor<Vec<u8>>,
    header: Header,
}

impl<R: Read + Seek> CopcEntryReader<'_, R> {
    /// Create a new Copc Entry reader
    pub fn new(mut read: R) -> Result<Self> {
        let header = Header::read_and_build_from(read.by_ref())?;
        let mut decompressor = LayeredPointRecordDecompressor::new(read);
        decompressor.set_fields_from(header.laz_vlr()?.items())?;
        let buffer = Cursor::new(Vec::new());
        Ok(Self {
            decompressor,
            buffer,
            header,
        })
    }

    /// Read all Points specified by entry
    pub fn read_entry_points(
        &mut self,
        entry: copc::Entry,
        points: &mut Vec<Point>,
    ) -> Result<u64> {
        let _off = self
            .decompressor
            .get_mut()
            .seek(SeekFrom::Start(entry.offset))?;
        points.reserve_exact(entry.point_count as usize);

        let resize = usize::try_from(
            entry.point_count as u64 * u64::from(self.header.point_format().len()),
        )?;
        self.buffer.get_mut().resize(resize, 0u8);
        self.decompressor.decompress_many(self.buffer.get_mut())?;
        self.buffer.set_position(0);
        points.reserve(entry.point_count as usize);

        for _ in 0..entry.point_count as usize {
            let point = raw::Point::read_from(&mut self.buffer, self.header.point_format())
                .map(|raw_point| Point::new(raw_point, self.header.transforms()))?;
            points.push(point);
        }
        Ok(entry.point_count as u64)
    }
}
#[cfg(test)]
mod tests {
    use crate::{reader::copc::CopcEntryReader, Reader};
    use std::{fs::File, io::BufReader};

    #[test]
    fn test_copc_read_autzen() {
        let copc_points = {
            let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
            let mut entry_reader = CopcEntryReader::new(file).unwrap();
            let root_entry = entry_reader
                .header
                .copc_hierarchy_evlr()
                .unwrap()
                .iter_entrys()
                .next()
                .unwrap();
            let mut points = Vec::new();
            let _p_num = entry_reader
                .read_entry_points(root_entry, &mut points)
                .unwrap();
            points
        };
        let mut laz_points = Vec::new();
        let _pnum = Reader::from_path("tests/data/autzen.copc.laz")
            .unwrap()
            .read_all_points_into(&mut laz_points)
            .unwrap();
        assert!(laz_points
            .iter()
            .zip(copc_points)
            .all(|(laz_point, copc_point)| laz_point.eq(&copc_point)));
    }
}
