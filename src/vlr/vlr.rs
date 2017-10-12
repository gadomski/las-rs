const HEADER_SIZE: u32 = 54;

/// A variable length record.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vlr {
    /// The user that created this record.
    ///
    /// This value is often an official, "registered" user_id, such as "LASF_Spec" or
    /// "LASF_Projection".
    pub user_id: String,
    /// This value specifies the type of record, and depends on the user id.
    pub record_id: u16,
    /// Textual description of these data.
    pub description: String,
    /// The data themselves.
    pub data: Vec<u8>,
}

impl Vlr {
    /// Returns the total length of this vlr, header and data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Vlr;
    /// let vlr = Vlr { ..Default::default() };
    /// assert_eq!(54, vlr.len());
    /// ```
    pub fn len(&self) -> u32 {
        HEADER_SIZE + self.data.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len() {
        let data = vec![0; 1];
        let vlr = Vlr {
            data: data,
            ..Default::default()
        };
        assert_eq!(55, vlr.len());
    }
}
