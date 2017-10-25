use Result;
use std::io::{Read, Write};

/// A raw VLR that maps directly onto the LAS specification.
#[derive(Debug, Default)]
#[allow(missing_docs)]
pub struct Vlr {
    pub reserved: u16,
    pub user_id: [u8; 16],
    pub record_id: u16,
    pub record_length_after_header: u16,
    pub description: [u8; 32],
    pub data: Vec<u8>,
}

impl Vlr {
    /// Reads a raw VLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::raw::Vlr;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(227));
    /// let vlr = Vlr::read_from(file).unwrap();
    /// ```
    pub fn read_from<R: Read>(mut read: R) -> Result<Vlr> {
        use byteorder::{LittleEndian, ReadBytesExt};
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
        Ok(Vlr {
            reserved: reserved,
            user_id: user_id,
            record_id: record_id,
            record_length_after_header: record_length_after_header,
            description: description,
            data: data,
        })
    }

    /// Writes a raw VLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// use las::raw::Vlr;
    /// let mut cursor = Cursor::new(Vec::new());
    /// let vlr = Vlr::default();
    /// vlr.write_to(cursor).unwrap();
    /// ```
    pub fn write_to<W: Write>(&self, mut write: W) -> Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
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

// TODO test if data length doesn't match record length after header.
// TODO test if reserved isn't zeros
