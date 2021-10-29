//! Variable length records, both extended and regular.

use std::io::{Read, Write};
use Result;

/// A raw variable length record.
#[derive(Debug, Default, PartialEq)]
pub struct Vlr {
    /// This value must be set to zero
    pub reserved: u16,

    /// The User ID field is ASCII character data that identifies the user which created the
    /// variable length record.
    ///
    /// It is possible to have many Variable Length Records from different sources with different
    /// User IDs. If the character data is less than 16 characters, the remaining data must be
    /// null. The User ID must be registered with the LAS specification managing body. The
    /// management of these User IDs ensures that no two individuals accidentally use the same User
    /// ID.
    pub user_id: [u8; 16],

    /// The Record ID is dependent upon the User ID.
    ///
    /// There can be 0 to 65,535 Record IDs for every User ID. The LAS specification manages its
    /// own Record IDs (User IDs owned by the specification), otherwise Record IDs will be managed
    /// by the owner of the given User ID. Thus each User ID is allowed to assign 0 to 65,535
    /// Record IDs in any manner they desire. Publicizing the meaning of a given Record ID is left
    /// to the owner of the given User ID. Unknown User ID/Record ID combinations should be
    /// ignored.
    pub record_id: u16,

    /// The record length is the number of bytes for the record after the end of the standard part
    /// of the header.
    ///
    /// Thus the entire record length is 54 bytes (the header size of the VLR) plus the number of
    /// bytes in the variable length portion of the record.
    pub record_length_after_header: RecordLength,

    /// Optional, null terminated text description of the data.
    ///
    /// Any remaining characters not used must be null.
    pub description: [u8; 32],

    #[allow(missing_docs)]
    pub data: Vec<u8>,
}

/// The length of the data in the vlr.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RecordLength {
    /// Vlrs use u16.
    Vlr(u16),
    /// Evlrs use u64.
    Evlr(u64),
}

impl Vlr {
    /// Reads a raw VLR or EVLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::raw::Vlr;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(227));
    /// // If the second parameter were true, it would be read as an extended vlr.
    /// let vlr = Vlr::read_from(file, false).unwrap();
    /// ```
    #[allow(clippy::field_reassign_with_default)]
    pub fn read_from<R: Read>(mut read: R, extended: bool) -> Result<Vlr> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let mut vlr = Vlr::default();
        vlr.reserved = read.read_u16::<LittleEndian>()?;
        read.read_exact(&mut vlr.user_id)?;
        vlr.record_id = read.read_u16::<LittleEndian>()?;
        vlr.record_length_after_header = if extended {
            RecordLength::Evlr(read.read_u64::<LittleEndian>()?)
        } else {
            RecordLength::Vlr(read.read_u16::<LittleEndian>()?)
        };
        read.read_exact(&mut vlr.description)?;
        vlr.data
            .resize(usize::from(vlr.record_length_after_header), 0);
        read.read_exact(&mut vlr.data)?;
        Ok(vlr)
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
        match self.record_length_after_header {
            RecordLength::Vlr(n) => write.write_u16::<LittleEndian>(n)?,
            RecordLength::Evlr(n) => write.write_u64::<LittleEndian>(n)?,
        }
        write.write_all(&self.description)?;
        write.write_all(&self.data)?;
        Ok(())
    }
}

impl From<RecordLength> for u64 {
    fn from(record_length: RecordLength) -> u64 {
        match record_length {
            RecordLength::Vlr(n) => u64::from(n),
            RecordLength::Evlr(n) => n,
        }
    }
}

impl From<RecordLength> for usize {
    fn from(record_length: RecordLength) -> usize {
        u64::from(record_length) as usize
    }
}

impl Default for RecordLength {
    fn default() -> RecordLength {
        RecordLength::Vlr(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip() {
        let vlr = Vlr::default();
        let mut cursor = Cursor::new(Vec::new());
        vlr.write_to(&mut cursor).unwrap();
        cursor.set_position(0);
        assert_eq!(vlr, Vlr::read_from(cursor, false).unwrap());
    }

    #[test]
    fn roundtrip_evlr() {
        let evlr = Vlr {
            record_length_after_header: RecordLength::Evlr(0),
            ..Default::default()
        };
        let mut cursor = Cursor::new(Vec::new());
        evlr.write_to(&mut cursor).unwrap();
        cursor.set_position(0);
        assert_eq!(evlr, Vlr::read_from(cursor, true).unwrap());
    }
}
