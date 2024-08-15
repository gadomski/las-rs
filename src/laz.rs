//! Utility functions for working with laszip compressed data.

use crate::{Error, Header, Result, Vlr};
use laz::{LazItemRecordBuilder, LazItemType, LazVlr};
use std::io::Cursor;

/// Returns true if this [Vlr] is the laszip Vlr.
///
/// # Examples
///
/// ```
/// #[cfg(feature = "laz")]
/// {
/// use las::{laz, Vlr};
///
/// let mut vlr = Vlr::default();
/// assert!(!laz::is_laszip_vlr(&vlr));
/// vlr.user_id = "laszip encoded".to_string();
/// vlr.record_id = 22204;
/// assert!(laz::is_laszip_vlr(&vlr));
/// }
/// ```
pub fn is_laszip_vlr(vlr: &Vlr) -> bool {
    vlr.user_id == LazVlr::USER_ID && vlr.record_id == LazVlr::RECORD_ID
}

impl Header {
    /// Adds a new laszip vlr to this header.
    ///
    /// Ensures that there's only one laszip vlr, as well.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    ///
    /// let mut header = Header::default();
    /// #[cfg(feature = "laz")]
    /// header.add_laz_vlr().unwrap();
    /// ```
    pub fn add_laz_vlr(&mut self) -> Result<()> {
        let point_format = self.point_format();
        let mut laz_items = LazItemRecordBuilder::new();
        if !point_format.is_extended {
            let _ = laz_items.add_item(LazItemType::Point10);

            if point_format.has_gps_time {
                let _ = laz_items.add_item(LazItemType::GpsTime);
            }

            if point_format.has_color {
                let _ = laz_items.add_item(LazItemType::RGB12);
            }

            if point_format.extra_bytes > 0 {
                let _ = laz_items.add_item(LazItemType::Byte(point_format.extra_bytes));
            }
        } else {
            let _ = laz_items.add_item(LazItemType::Point14);

            if point_format.has_color {
                // Point format 7 & 8 both have RGB
                if point_format.has_nir {
                    let _ = laz_items.add_item(LazItemType::RGBNIR14);
                } else {
                    let _ = laz_items.add_item(LazItemType::RGB14);
                }
            }
            if point_format.extra_bytes > 0 {
                let _ = laz_items.add_item(LazItemType::Byte14(point_format.extra_bytes));
            }
        }
        let laz_vlr = LazVlr::from_laz_items(laz_items.build());
        let mut cursor = Cursor::new(Vec::<u8>::new());
        laz_vlr.write_to(&mut cursor)?;
        let vlr = Vlr {
            user_id: LazVlr::USER_ID.to_owned(),
            record_id: LazVlr::RECORD_ID,
            description: LazVlr::DESCRIPTION.to_owned(),
            data: cursor.into_inner(),
        };
        self.vlrs.push(vlr);
        Ok(())
    }

    /// Returns header's [LazVlr], or `None` if none is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Header;
    ///
    /// let mut header = Header::default();
    ///
    /// #[cfg(feature = "laz")]
    /// {
    /// assert!(header.laz_vlr().is_none());
    /// header.add_laz_vlr();
    /// assert!(header.laz_vlr().is_some());
    /// }
    /// ```
    pub fn laz_vlr(&self) -> Option<LazVlr> {
        self.vlrs
            .iter()
            .find(|vlr| is_laszip_vlr(vlr))
            .and_then(|vlr| vlr.try_into().ok())
    }
}

impl TryFrom<&Vlr> for LazVlr {
    type Error = Error;

    fn try_from(vlr: &Vlr) -> Result<LazVlr> {
        LazVlr::from_buffer(&vlr.data).map_err(Error::from)
    }
}
