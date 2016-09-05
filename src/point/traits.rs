use std::io::{self, Read};

use byteorder::{LittleEndian, ReadBytesExt};

use {Error, Result};
use point::{Classification, Color, Format, NumberOfReturns, Point, ReturnNumber, ScanDirection,
            utils};
use utils::{LinearTransform, Triple};
use version::Version;

/// Reads a point.
pub trait ReadPoint {
    /// Reads a point.
    ///
    /// If there is no point to read, returns `Ok(None)`.
    fn read_point(&mut self,
                  transforms: Triple<LinearTransform>,
                  format: Format,
                  version: Version,
                  extra_bytes: u16)
                  -> Result<Option<Point>>;
}

impl<R: Read> ReadPoint for R {
    fn read_point(&mut self,
                  transforms: Triple<LinearTransform>,
                  format: Format,
                  version: Version,
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
        // TODO classiciations shouldn't care about version, really?
        let classification = Classification::from(try!(self.read_u8()), version);
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
