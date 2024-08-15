use super::WritePoint;
use crate::{Header, Point, Result};
use std::io::{Seek, Write};

pub(crate) struct PointWriter<W: Write + Seek> {
    write: W,
    header: Header,
}

impl<W: Write + Seek> PointWriter<W> {
    pub(crate) fn new(write: W, header: Header) -> PointWriter<W> {
        PointWriter { write, header }
    }
}

impl<W: Write + Seek + Send> WritePoint<W> for PointWriter<W> {
    fn write_point(&mut self, point: Point) -> Result<()> {
        self.header.add_point(&point);
        point
            .into_raw(self.header.transforms())
            .and_then(|raw_point| raw_point.write_to(&mut self.write, self.header.point_format()))
    }

    fn into_inner(self: Box<Self>) -> W {
        self.write
    }

    fn get_mut(&mut self) -> &mut W {
        &mut self.write
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn header_mut(&mut self) -> &mut Header {
        &mut self.header
    }

    fn done(&mut self) -> Result<()> {
        Ok(())
    }
}
