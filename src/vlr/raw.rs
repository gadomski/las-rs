use {Error, Result, Vlr};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use std::u16;
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
        user_id.as_mut().from_las_str(&self.user_id)?;
        let mut description = [0; 32];
        description.as_mut().from_las_str(&self.description)?;
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
    /// Reads a raw VLR.
    ///
    /// # Examples
    ///
    /// `Read` implements `ReadRawVlr`.
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::vlr::RawVlr;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(227));
    /// let vlr = RawVlr::read_from(file).unwrap();
    /// ```
    pub fn read_from<R: Read>(mut read: R) -> Result<RawVlr> {
        let reserved = read.read_u16::<LittleEndian>()?;
        let mut user_id = [0; 16];
        read.read_exact(&mut user_id)?;
        let record_id = read.read_u16::<LittleEndian>()?;
        let record_length_after_header = read.read_u16::<LittleEndian>()?;
        let mut description = [0; 32];
        read.read_exact(&mut description)?;
        let mut data = Vec::with_capacity(record_length_after_header as usize);
        read.take(record_length_after_header as u64).read_to_end(
            &mut data,
        )?;
        Ok(RawVlr {
            reserved: reserved,
            user_id: user_id,
            record_id: record_id,
            record_length_after_header: record_length_after_header,
            description: description,
            data: data,
        })
    }

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
            user_id: self.user_id.as_ref().to_las_str()?.to_string(),
            record_id: self.record_id,
            description: self.description.as_ref().to_las_str()?.to_string(),
            data: self.data,
        })
    }

    /// Writes a raw VLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::vlr::RawVlr;
    /// let mut cursor = Cursor::new(Vec::new());
    /// let raw_vlr = RawVlr::default();
    /// raw_vlr.write_to(cursor).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W) -> Result<()> {
        write.write_u16::<LittleEndian>(self.reserved)?;
        write.write_all(&self.user_id)?;
        write.write_u16::<LittleEndian>(self.record_id)?;
        write.write_u16::<LittleEndian>(
            self.record_length_after_header,
        )?;
        write.write_all(&self.description)?;
        write.write_all(&self.data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use Vlr;
    use std::u16;

    #[test]
    fn too_long_vlr_data() {
        let data = vec![0; u16::MAX as usize + 1];
        let vlr = Vlr {
            data: data,
            ..Default::default()
        };
        assert!(vlr.to_raw_vlr().is_err());
    }
}
