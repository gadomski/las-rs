//! Variable length records are used to store additional metadata not defined in the header.
//!
//! Variable length records (VLRs) can be "regular" or "extended". "Regular" vlrs are stored right
//! after the header, before the point records. "Extended" vlrs (EVLRs) are stored at the end of
//! the file, after the point records.
//!
//! Vlrs contain arbitrary data:
//!
//! ```
//! use las::Vlr;
//! let mut vlr = Vlr::default();
//! vlr.user_id = "gadomski".to_string();
//! vlr.record_id = 42;
//! vlr.description = "Some really important data".to_string();
//! vlr.data = vec![1, 2, 3];
//! ```
//!
//! Extended variable length records can be created with the `extended` method. Note that, when
//! possible, evlrs will be downcasted to vlrs when writing to versions that don't support evlrs.
//!
//! ```
//! use las::{Vlr, Builder};
//! let mut builder = Builder::from((1, 4));
//!
//! let evlr = Vlr::extended();
//! builder.vlrs.push(evlr);
//! let header = builder.clone().into_header().unwrap();
//! assert_eq!(1, header.evlrs().len());
//!
//! builder.version = (1, 2).into(); // las 1.2 doesn't support evlrs
//! let header = builder.into_header().unwrap();
//! assert_eq!(0, header.evlrs().len());
//! assert_eq!(1, header.vlrs().len());
//! ```

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

    is_extended: bool,
}

impl Vlr {
    /// Creates a default extended vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let vlr = Vlr::extended();
    /// assert!(vlr.is_extended());
    /// ```
    pub fn extended() -> Vlr {
        Vlr {
            is_extended: true,
            ..Default::default()
        }
    }

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
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let raw_vlr =  Vlr::default().into_raw().unwrap();
    /// let raw_evlr =  Vlr::extended().into_raw().unwrap();
    /// ```
    pub fn into_raw(self) -> Result<raw::Vlr> {
        use utils::FromLasStr;

        let mut user_id = [0; 16];
        user_id.as_mut().from_las_str(&self.user_id)?;
        let mut description = [0; 32];
        description.as_mut().from_las_str(&self.description)?;
        Ok(raw::Vlr {
            reserved: 0,
            user_id: user_id,
            record_id: self.record_id,
            record_length_after_header: self.record_length_after_header()?,
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

    /// Returns true if the data of this vlr is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let mut vlr = Vlr::default();
    /// assert!(vlr.is_empty());
    /// vlr.data = vec![42];
    /// assert!(!vlr.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Mark this vlr as not extended (regular).
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let mut vlr = Vlr::extended();
    /// assert!(vlr.is_extended());
    /// vlr.unextend();
    /// assert!(!vlr.is_extended());
    /// ```
    pub fn unextend(&mut self) {
        self.is_extended = false;
    }

    /// Mark this vlr as extended.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let mut vlr = Vlr::default();
    /// assert!(!vlr.is_extended());
    /// vlr.extend();
    /// assert!(vlr.is_extended());
    /// ```
    pub fn extend(&mut self) {
        self.is_extended = true;
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
    /// let vlr = Vlr::default();
    /// assert!(!vlr.is_extended());
    /// let vlr = Vlr::extended();
    /// assert!(vlr.is_extended());
    ///
    /// let mut vlr = Vlr::default();
    /// vlr.data = vec![0; u16::MAX as usize + 1];
    /// assert!(vlr.is_extended());
    /// ```
    pub fn is_extended(&self) -> bool {
        self.is_extended || self.has_large_data()
    }

    /// Returns true if this vlr *must* be extended due to large data size.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// use std::u16;
    /// let mut evlr = Vlr::extended();
    /// assert!(!evlr.has_large_data());
    /// evlr.data = vec![0; u16::MAX as usize + 1 ];
    /// assert!(evlr.has_large_data());
    pub fn has_large_data(&self) -> bool {
        use std::u16;
        self.data.len() > u16::MAX as usize
    }

    fn record_length_after_header(&self) -> Result<raw::vlr::RecordLength> {
        if self.is_extended {
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
        assert!(vlr.into_raw().is_err());
    }
}
