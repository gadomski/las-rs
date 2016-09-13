use std::io::{Read, Write};
use std::u16;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use {Error, Result, Vlr};
use utils::{FromLasStr, ToLasStr};

/// A raw VLR that maps directly onto the LAS specification.
#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct RawVlr {
    pub reserved: u16,
    pub user_id: [u8; 16],
    pub record_id: u16,
    pub record_length_after_header: u16,
    pub description: [u8; 32],
    pub data: Vec<u8>,
}

impl Vlr {
    /// Converts this vlr to a raw vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Vlr;
    /// let raw_vlr =  Vlr { ..Default::default() }.to_raw_vlr().unwrap();
    /// ```
    pub fn to_raw_vlr(&self) -> Result<RawVlr> {
        if self.data.len() > u16::MAX as usize {
            return Err(Error::VlrDataTooLong(self.data.len()));
        }
        let mut user_id = [0; 16];
        try!(user_id.as_mut().from_las_str(&self.user_id));
        let mut description = [0; 32];
        try!(description.as_mut().from_las_str(&self.description));
        Ok(RawVlr {
            reserved: 0,
            user_id: user_id,
            record_id: self.record_id,
            record_length_after_header: self.data.len() as u16,
            description: description,
            data: self.data.clone(),
        })
    }
}

impl RawVlr {
    /// Converts this raw vlr into a vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::vlr::RawVlr;
    /// let vlr = RawVlr { ..Default::default() }.into_vlr().unwrap();
    /// ```
    pub fn into_vlr(self) -> Result<Vlr> {
        Ok(Vlr {
            user_id: try!(self.user_id.as_ref().to_las_str()).to_string(),
            record_id: self.record_id,
            description: try!(self.description.as_ref().to_las_str()).to_string(),
            data: self.data,
        })
    }
}

/// Reads a raw VLR.
pub trait ReadRawVlr {
    /// Reads a raw VLR.
    ///
    /// # Examples
    ///
    /// `Read` implements `ReadRawVlr`.
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::vlr::ReadRawVlr;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(227));
    /// let vlr = file.read_raw_vlr().unwrap();
    /// ```
    fn read_raw_vlr(&mut self) -> Result<RawVlr>;
}

impl<R: Read> ReadRawVlr for R {
    fn read_raw_vlr(&mut self) -> Result<RawVlr> {
        let reserved = try!(self.read_u16::<LittleEndian>());
        let mut user_id = [0; 16];
        try!(self.read_exact(&mut user_id));
        let record_id = try!(self.read_u16::<LittleEndian>());
        let record_length_after_header = try!(self.read_u16::<LittleEndian>());
        let mut description = [0; 32];
        try!(self.read_exact(&mut description));
        let mut data = Vec::with_capacity(record_length_after_header as usize);
        try!(self.take(record_length_after_header as u64).read_to_end(&mut data));
        Ok(RawVlr {
            reserved: reserved,
            user_id: user_id,
            record_id: record_id,
            record_length_after_header: record_length_after_header,
            description: description,
            data: data,
        })
    }
}

/// Writes a raw VLR.
pub trait WriteRawVlr {
    /// Writes a raw VLR.
    ///
    /// # Examples
    ///
    /// `Write` implements `WriteRawVlr`.
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::vlr::WriteRawVlr;
    /// let mut cursor = Cursor::new(Vec::new());
    /// cursor.write_raw_vlr(&Default::default()).unwrap();
    /// ```
    fn write_raw_vlr(&mut self, raw_vlr: &RawVlr) -> Result<()>;
}

impl<W: Write> WriteRawVlr for W {
    fn write_raw_vlr(&mut self, raw_vlr: &RawVlr) -> Result<()> {
        try!(self.write_u16::<LittleEndian>(raw_vlr.reserved));
        try!(self.write_all(&raw_vlr.user_id));
        try!(self.write_u16::<LittleEndian>(raw_vlr.record_id));
        try!(self.write_u16::<LittleEndian>(raw_vlr.record_length_after_header));
        try!(self.write_all(&raw_vlr.description));
        try!(self.write_all(&raw_vlr.data));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::u16;

    use Vlr;

    #[test]
    fn too_long_vlr_data() {
        let data = vec![0; u16::MAX as usize + 1];
        let vlr = Vlr { data: data, ..Default::default() };
        assert!(vlr.to_raw_vlr().is_err());
    }
}
