use std::io::{self, Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use {Error, Result};
use point::{Classification, Color, Format, NumberOfReturns, Point, ReturnNumber, ScanDirection,
            utils};
use utils::{LinearTransform, Triple};

/// Reads a point.
pub trait ReadPoint {
    /// Reads a point.
    ///
    /// If there is no point to read, returns `Ok(None)`.
    fn read_point(&mut self,
                  transforms: Triple<LinearTransform>,
                  format: Format,
                  extra_bytes: u16)
                  -> Result<Option<Point>>;
}

impl<R: Read> ReadPoint for R {
    fn read_point(&mut self,
                  transforms: Triple<LinearTransform>,
                  format: Format,
                  extra_bytes: u16)
                  -> Result<Option<Point>> {
        let x = match self.read_i32::<LittleEndian>() {
            Ok(n) => transforms.x.direct(n),
            Err(err) => {
                return if err.kind() == io::ErrorKind::UnexpectedEof {
                    Ok(None)
                } else {
                    Err(Error::from(err))
                }
            }
        };
        let y = transforms.y.direct(try!(self.read_i32::<LittleEndian>()));
        let z = transforms.z.direct(try!(self.read_i32::<LittleEndian>()));
        let intensity = try!(self.read_u16::<LittleEndian>());
        let byte = try!(self.read_u8());
        let return_number = ReturnNumber::from(byte);
        let number_of_returns = NumberOfReturns::from(byte);
        let scan_direction = ScanDirection::from(byte);
        let edge_of_flight_line = utils::edge_of_flight_line(byte);
        let classification = Classification::from(try!(self.read_u8()));
        let scan_angle_rank = try!(self.read_i8());
        let user_data = try!(self.read_u8());
        let point_source_id = try!(self.read_u16::<LittleEndian>());
        let gps_time = if format.has_gps_time() {
            Some(try!(self.read_f64::<LittleEndian>()))
        } else {
            None
        };
        let color = if format.has_color() {
            let red = try!(self.read_u16::<LittleEndian>());
            let green = try!(self.read_u16::<LittleEndian>());
            let blue = try!(self.read_u16::<LittleEndian>());
            Some(Color {
                red: red,
                green: green,
                blue: blue,
            })
        } else {
            None
        };
        let mut bytes = Vec::new();
        if extra_bytes > 0 {
            try!(self.take(extra_bytes as u64).read_to_end(&mut bytes));
        }
        Ok(Some(Point {
            x: x,
            y: y,
            z: z,
            intensity: intensity,
            return_number: return_number,
            number_of_returns: number_of_returns,
            scan_direction: scan_direction,
            edge_of_flight_line: edge_of_flight_line,
            classification: classification,
            scan_angle_rank: scan_angle_rank,
            user_data: user_data,
            point_source_id: point_source_id,
            gps_time: gps_time,
            color: color,
            extra_bytes: bytes,
        }))
    }
}

/// Writes a point
pub trait WritePoint {
    /// Writes a point.
    fn write_point(&mut self,
                   point: &Point,
                   transforms: Triple<LinearTransform>,
                   format: Format,
                   extra_bytes: u16)
                   -> Result<()>;
}

impl<W: Write> WritePoint for W {
    fn write_point(&mut self,
                   point: &Point,
                   transforms: Triple<LinearTransform>,
                   format: Format,
                   extra_bytes: u16)
                   -> Result<()> {
        try!(self.write_i32::<LittleEndian>(transforms.x.inverse(point.x)));
        try!(self.write_i32::<LittleEndian>(transforms.y.inverse(point.y)));
        try!(self.write_i32::<LittleEndian>(transforms.z.inverse(point.z)));
        try!(self.write_u16::<LittleEndian>(point.intensity));
        try!(self.write_u8(u8::from(point.return_number) | u8::from(point.number_of_returns) |
                           u8::from(point.scan_direction) |
                           utils::edge_of_flight_line_u8(point.edge_of_flight_line)));
        try!(self.write_u8(point.classification.into()));
        try!(self.write_i8(point.scan_angle_rank));
        try!(self.write_u8(point.user_data));
        try!(self.write_u16::<LittleEndian>(point.point_source_id));
        if format.has_gps_time() {
            match point.gps_time {
                Some(time) => try!(self.write_f64::<LittleEndian>(time)),
                None => return Err(Error::MissingGpsTime(format, (*point).clone())),
            }
        }
        if format.has_color() {
            match point.color {
                Some(Color { red, green, blue }) => {
                    try!(self.write_u16::<LittleEndian>(red));
                    try!(self.write_u16::<LittleEndian>(green));
                    try!(self.write_u16::<LittleEndian>(blue));
                }
                None => return Err(Error::MissingColor(format, (*point).clone())),
            }
        }
        // TODO count mismatch?
        if extra_bytes > 0 {
            try!(self.write_all(point.extra_bytes.as_slice()));
        }
        Ok(())
    }
}
