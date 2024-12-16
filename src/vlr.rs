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
//! ```
//! use las::{Vlr, Builder};
//! let mut builder = Builder::from((1, 4));
//!
//! builder.evlrs.push(Vlr::default());
//! let header = builder.clone().into_header().unwrap();
//! assert_eq!(1, header.evlrs().len());
//!
//! builder.version = (1, 2).into(); // las 1.2 doesn't support evlrs
//! let header = builder.into_header().unwrap();
//! assert_eq!(0, header.evlrs().len());
//! assert_eq!(1, header.vlrs().len());
//! ```

use crate::{raw, Error, Result};

const REGULAR_HEADER_SIZE: usize = 54;
const EXTENDED_HEADER_SIZE: usize = 60;

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
    /// let vlr = Vlr::new(raw_vlr);
    /// ```
    pub fn new(raw_vlr: raw::Vlr) -> Vlr {
        use crate::utils::AsLasStr;
        Vlr {
            user_id: raw_vlr.user_id.as_ref().as_las_string_lossy(),
            record_id: raw_vlr.record_id,
            description: raw_vlr.description.as_ref().as_las_string_lossy(),
            data: raw_vlr.data,
        }
    }

    /// Converts this vlr to a raw vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// let raw_vlr =  Vlr::default().into_raw(false).unwrap();
    /// let raw_evlr =  Vlr::default().into_raw(true).unwrap();
    /// ```
    pub fn into_raw(self, is_extended: bool) -> Result<raw::Vlr> {
        use crate::utils::FromLasStr;

        let mut user_id = [0; 16];
        user_id.as_mut().from_las_str(&self.user_id)?;
        let mut description = [0; 32];
        description.as_mut().from_las_str(&self.description)?;
        Ok(raw::Vlr {
            reserved: 0,
            user_id,
            record_id: self.record_id,
            record_length_after_header: self.record_length_after_header(is_extended)?,
            description,
            data: self.data,
        })
    }

    /// Returns the total length of this vlr, header and data.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    ///
    /// let mut vlr = Vlr::default();
    /// assert_eq!(54, vlr.len(false));
    /// assert_eq!(60, vlr.len(true));
    /// vlr.data = vec![0];
    /// assert_eq!(55, vlr.len(false));
    /// ```
    pub fn len(&self, is_extended: bool) -> usize {
        self.data.len()
            + if is_extended {
                EXTENDED_HEADER_SIZE
            } else {
                REGULAR_HEADER_SIZE
            }
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

    /// Returns true if this vlr *must* be extended due to large data size.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    /// use std::u16;
    /// let mut vlr = Vlr::default();
    /// assert!(!vlr.has_large_data());
    /// vlr.data = vec![0; u16::MAX as usize + 1 ];
    /// assert!(vlr.has_large_data());
    pub fn has_large_data(&self) -> bool {
        self.data.len() > u16::MAX as usize
    }

    fn record_length_after_header(&self, is_extended: bool) -> Result<raw::vlr::RecordLength> {
        if is_extended {
            Ok(raw::vlr::RecordLength::Evlr(self.data.len() as u64))
        } else if self.data.len() > u16::MAX as usize {
            Err(Error::VlrTooLong(self.data.len()))
        } else {
            Ok(raw::vlr::RecordLength::Vlr(self.data.len() as u16))
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
            data,
            ..Default::default()
        };
        assert_eq!(55, vlr.len(false));
        assert_eq!(61, vlr.len(true));
    }

    #[test]
    fn too_long() {
        use std::u16;
        let data = vec![0; u16::MAX as usize + 1];
        let vlr = Vlr {
            data,
            ..Default::default()
        };
        assert!(vlr.into_raw(false).is_err());
    }

    #[test]
    fn allow_non_ascii_user_id() {
        let raw_vlr = raw::Vlr {
            user_id: [194, 174, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let vlr = Vlr::new(raw_vlr);
        assert_eq!("®", vlr.user_id);
    }

    #[test]
    fn allow_non_ascii_description() {
        let raw_vlr = raw::Vlr {
            description: [
                194, 174, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0,
            ],
            ..Default::default()
        };
        let vlr = Vlr::new(raw_vlr);
        assert_eq!("®", vlr.description);
    }
}
