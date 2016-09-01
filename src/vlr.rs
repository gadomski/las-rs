const HEADER_LENGTH: u32 = 54;

/// Variable length record.
#[derive(Clone, Debug)]
pub struct Vlr {
    /// ASCII data which identifies the user assocaiated with the record.
    ///
    /// These are registered with ASPRS.
    pub user_id: [u8; 16],
    /// Dependent on user id.
    pub record_id: u16,
    /// The length of the record after the standard header.
    pub record_length: u16,
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
