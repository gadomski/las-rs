//! Variable length records.
//!
//! This includes regular and extended.

use {Result, raw};

const HEADER_SIZE: usize = 54;

quick_error! {
    /// Vlr-specific errors.
    #[derive(Debug, Clone, Copy)]
    pub enum Error {
        /// The vlr data is too long.
        TooLong(len: usize) {
            description("The vlr is too long")
            display("the vlr is too long: {}", len)
        }
    }
}

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
    /// Should this vlr be written "extended", i.e. at the end of the file.
    ///
    /// Note that not all las versions support extended vlrs, and extra-long vlrs might be written
    /// as extended even if this flag is not set.
    pub is_extended: bool,
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
        use utils::AsLasStr;
        Ok(Vlr {
            user_id: raw_vlr.user_id.as_ref().as_las_str()?.to_string(),
            record_id: raw_vlr.record_id,
            description: raw_vlr.description.as_ref().as_las_str()?.to_string(),
            is_extended: raw_vlr.is_extended(),
            data: raw_vlr.data,
        })
    }

    /// Converts this vlr to a raw vlr.
    ///
    /// The second argument works like this:
    ///
    /// - If `Some(false)`, create a normal vlr.
    /// - If `Some(true)`, create an extended vlr.
    /// - If `None`, fall back to the default as defined by the `extended` flag on this vlr.
    ///
    /// Note that you can just pass `true` or `false` and it will be converted to the option type.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let raw_vlr =  Vlr::default().into_raw(false).unwrap();
    /// let raw_evlr =  Vlr::default().into_raw(true).unwrap();
    /// let raw_vlr2 =  Vlr::default().into_raw(None).unwrap();
    /// ```
    pub fn into_raw<T>(self, force_extended: T) -> Result<raw::Vlr>
    where
        T: Into<Option<bool>>,
    {
        use utils::FromLasStr;

        let extended = force_extended.into().unwrap_or(self.is_extended);
        let mut user_id = [0; 16];
        user_id.as_mut().from_las_str(&self.user_id)?;
        let mut description = [0; 32];
        description.as_mut().from_las_str(&self.description)?;
        Ok(raw::Vlr {
            reserved: 0,
            user_id: user_id,
            record_id: self.record_id,
            record_length_after_header: self.record_length_after_header(extended)?,
            description: description,
            data: self.data,
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
    pub fn len(&self) -> usize {
        self.data.len() + HEADER_SIZE
    }

    /// Returns true if this vlr is extended.
    ///
    /// True either if the flag is set, or the data is too long for a normal vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::u16;
    /// use las::Vlr;
    /// let mut vlr = Vlr::default();
    /// assert!(!vlr.is_extended());
    /// vlr.is_extended = true;
    /// assert!(vlr.is_extended());
    ///
    /// vlr.is_extended = false;
    /// vlr.data = vec![0; u16::MAX as usize + 1];
    /// assert!(vlr.is_extended());
    /// ```
    pub fn is_extended(&self) -> bool {
        use std::u16;
        self.is_extended || self.data.len() > u16::MAX as usize
    }

    fn record_length_after_header(&self, extended: bool) -> Result<raw::vlr::RecordLength> {
        if extended {
            Ok(raw::vlr::RecordLength::Evlr(self.data.len() as u64))
        } else {
            use std::u16;
            if self.data.len() > u16::MAX as usize {
                Err(Error::TooLong(self.data.len()).into())
            } else {
                Ok(raw::vlr::RecordLength::Vlr(self.data.len() as u16))
            }
        }
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
    fn too_long() {
        use std::u16;
        let data = vec![0; u16::MAX as usize + 1];
        let vlr = Vlr {
            data: data,
            ..Default::default()
        };
        assert!(vlr.into_raw(false).is_err());
    }
}
