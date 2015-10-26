//! Variable length records.

use std::io::Read;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

use Error;
use Result;
use io::LasStringExt;

#[derive(Debug, Default)]
pub struct Vlr {
    pub reserved: u16,
    pub user_id: String,
    pub record_id: u16,
    pub record_length_after_header: u16,
    pub description: String,
    pub body: Vec<u8>,
}

impl Vlr {
    /// Reads `n` `Vlr`s from a `Read`.
    ///
    /// # Example
    ///
    /// ```
    /// # use las::vlr::Vlr;
    /// use std::fs::File;
    /// use std::io::{Seek, SeekFrom};
    /// let ref mut reader = File::open("data/1.2_0.las").unwrap();
    /// reader.seek(SeekFrom::Start(227));
    /// let vlrs = Vlr::read_n_from(reader, 2).unwrap();
    /// assert_eq!(2, vlrs.len());
    /// ```
    pub fn read_n_from<R: Read>(reader: &mut R, n: usize) -> Result<Vec<Vlr>> {
        let mut vlrs: Vec<Vlr> = Vec::new();
        for _ in 0..n {
            vlrs.push(try!(Vlr::read_from(reader)));
        }
        Ok(vlrs)
    }

    /// Reads a `Vlr` from a `Read`.
    ///
    /// # Example
    ///
    /// ```
    /// # use las::vlr::Vlr;
    /// use std::fs::File;
    /// use std::io::{Seek, SeekFrom};
    /// let ref mut reader = File::open("data/1.2_0.las").unwrap();
    /// reader.seek(SeekFrom::Start(227));
    /// let vlr = Vlr::read_from(reader);
    /// ```
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Vlr> {
        let mut vlr: Vlr = Default::default();
        vlr.reserved = try!(reader.read_u16::<LittleEndian>());
        vlr.user_id = try!(reader.read_las_string(16));
        vlr.record_id = try!(reader.read_u16::<LittleEndian>());
        vlr.record_length_after_header = try!(reader.read_u16::<LittleEndian>());
        vlr.description = try!(reader.read_las_string(32));
        let num_read = try!(reader.take(vlr.record_length_after_header as u64)
                                  .read_to_end(&mut vlr.body));
        if num_read != vlr.record_length_after_header as usize {
            return Err(Error::ReadError(format!("Tried to take {} bytes, only took {}",
                                                vlr.record_length_after_header,
                                                num_read)));
        }
        Ok(vlr)
    }
}
