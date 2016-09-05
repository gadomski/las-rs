use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};

use Result;

const HEADER_LENGTH: u32 = 54;

/// Variable length record.
#[derive(Clone, Debug, Default)]
pub struct Vlr {
    /// ASCII data which identifies the user assocaiated with the record.
    ///
    /// These are registered with ASPRS.
    pub user_id: [u8; 16],
    /// Dependent on user id.
    pub record_id: u16,
    /// Textual description of the VLR>
    pub description: [u8; 32],
    /// The data contained within the vlr.
    pub data: Vec<u8>,
}

impl Vlr {
    /// Returns the total length of the VLR.
    pub fn len(&self) -> u32 {
        self.data.len() as u32 + HEADER_LENGTH
    }
}

pub trait ReadVlr {
    fn read_vlr(&mut self) -> Result<Vlr>;
}

impl<R: Read> ReadVlr for R {
    fn read_vlr(&mut self) -> Result<Vlr> {
        try!(self.read_u16::<LittleEndian>()); // reserved
        let mut user_id = [0; 16];
        try!(self.read_exact(&mut user_id));
        let record_id = try!(self.read_u16::<LittleEndian>());
        let record_length = try!(self.read_u16::<LittleEndian>());
        let mut description = [0; 32];
        try!(self.read_exact(&mut description));
        let mut data = Vec::with_capacity(record_length as usize);
        try!(self.take(record_length as u64).read_to_end(&mut data));
        Ok(Vlr {
            user_id: user_id,
            record_id: record_id,
            description: description,
            data: data,
        })
    }
}
