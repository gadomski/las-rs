use super::ReadPoints;
use crate::{raw, Header, Point, PointCloud, Result};
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
    fn read_point(&mut self) -> Result<Option<Point>> {
        if self.index < self.header.number_of_points() {
            self.index += 1;
            raw::Point::read_from(&mut self.read, self.header.point_format())
                .map(|p| Point::new(p, self.header.transforms()))
                .map(Some)
        } else {
            Ok(None)
        }
    }

    fn read_points(&mut self, n: u64, points: &mut Vec<Point>) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);
        if let Ok(n) = usize::try_from(n) {
            points.reserve(n);
        }
        let mut count = 0;
        for _ in 0..n {
            if let Some(point) = self.read_point()? {
                points.push(point);
                count += 1;
            } else {
                break;
            }
        }
        Ok(count)
    }

    fn read_points_into_cloud(&mut self, n: u64, cloud: &mut PointCloud) -> Result<u64> {
        let points_left = self.header.number_of_points() - self.index;
        let n = points_left.min(n);
        let n_usize = usize::try_from(n)?;
        let slab = cloud.resize_for(n_usize);
        self.read.read_exact(slab)?;
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
