use {Result, raw};

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
    /// Creates a vlr from a raw vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{Vlr, raw};
    /// let raw_vlr = raw::Vlr::default();
    /// let vlr = Vlr::new(raw_vlr).unwrap();
    /// ```
    pub fn new(raw_vlr: raw::Vlr) -> Result<Vlr> {
        use utils::ToLasStr;
        Ok(Vlr {
            user_id: raw_vlr.user_id.as_ref().to_las_str()?.to_string(),
            record_id: raw_vlr.record_id,
            description: raw_vlr.description.as_ref().to_las_str()?.to_string(),
            data: raw_vlr.data,
        })
    }

    /// Converts this vlr to a raw vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let raw_vlr =  Vlr::default().to_raw().unwrap();
    /// ```
    pub fn to_raw(&self) -> Result<raw::Vlr> {
        use Error;
        use std::u16;
        use utils::FromLasStr;

        if self.data.len() > u16::MAX as usize {
            return Err(Error::VlrDataTooLong(self.data.len()));
        }
        let mut user_id = [0; 16];
        user_id.as_mut().from_las_str(&self.user_id)?;
        let mut description = [0; 32];
        description.as_mut().from_las_str(&self.description)?;
        Ok(raw::Vlr {
            reserved: 0,
            user_id: user_id,
            record_id: self.record_id,
            record_length_after_header: self.data.len() as u16,
            description: description,
            data: self.data.clone(),
        })
    }

    /// Returns the total length of this vlr, header and data.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let vlr = Vlr::default();
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

    #[test]
    fn too_long_vlr_data() {
        use std::u16;
        let data = vec![0; u16::MAX as usize + 1];
        let vlr = Vlr {
            data: data,
            ..Default::default()
        };
        assert!(vlr.to_raw().is_err());
    }
}
