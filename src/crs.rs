//! Module for handeling Coordinate Reference System data in a headers variable length records
//!
//! CRSes are stored either as WKT or in GeoTiff
//! [get_epsg_crs] parses the CRS data to EPSG code(s)
//!
//! Only WKT is supported for writing CRS data to a header
//!
//! [get_epsg_crs]: Header::get_epsg_crs

use byteorder::{LittleEndian, ReadBytesExt};
use log::{log, Level};

use std::io::{Cursor, Seek, SeekFrom};

use crate::{Error, Header, Result};

/// Horizontal and optional vertical CRS given by EPSG code(s)
#[derive(Debug, Clone, Copy)]
pub struct EpsgCrs {
    /// EPSG code for the horizontal CRS
    pub horizontal: u16,
    /// Optional EPSG code for the vertical CRS
    pub vertical: Option<u16>,
}

impl Header {
    /// Parse the EPSG coordinate reference system (CRSes) code(s) from the header.
    /// Las stores CRS-info in (E)VLRs either as Well Known Text (WKT) or in GeoTIff-format
    ///
    /// Most (not all!) CRSes used for Aerial Lidar has an associated EPSG code.
    /// Use this function to try and parse the EPSG code(s) from the VLR data.
    ///
    /// WKT takes precedence over GeoTiff in this function, but they should not co-exist
    ///
    /// Just because this function fails does not mean that no CRS-data is availible!
    /// Use functions [Self::get_wkt_crs] or [Self::get_geotiff_crs] to get all data stored in the CRS-(E)VLRs
    ///
    /// Parsing code(s) from WKT-CRS v1 or v2 or GeoTiff U16-data is supported
    ///
    /// The CRS is returend in a Result<Option<[EpsgCrs]>, Error>
    /// [EpsgCrs] has the fields horizontal, which is a u16 EPSG code, and vertical, which is an optional u16 EPSG code.
    ///
    /// The validity of the extracted code is not checked.
    /// Use the crs-definitions crate for checking the validity of a horizontal EPSG code.
    ///
    /// # Example
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("lidar.las").expect("Cannot open reader");
    /// let crs = reader.header().get_epsg_crs().expect("Cannot parse EPSG code(s) from the CRS-(E)VLRs");
    /// ```
    pub fn get_epsg_crs(&self) -> Result<Option<EpsgCrs>> {
        let wkt = self.get_wkt_crs();

        // warn about header and VLR inconsistencies
        if wkt.is_some() && !self.has_wkt_crs() {
            log!(
                Level::Warn,
                "WKT CRS (E)VLR found, but header says it does not exist"
            );
        } else if wkt.is_none() && self.has_wkt_crs() {
            log!(
                Level::Warn,
                "No WKT CRS (E)VLR found, but header says it exists"
            );
        }

        if let Some(wkt) = wkt {
            return get_epsg_from_wkt_crs(wkt);
        }

        let geotiff = self.get_geotiff_crs()?;

        if self.has_wkt_crs() && geotiff.is_some() {
            log!(
                Level::Warn,
                "Only Geotiff CRS (E)VLRs found, but header says WKT exists"
            );
        }

        if let Some(geotiff) = geotiff {
            return get_epsg_from_geotiff_crs(geotiff);
        }
        Ok(None)
    }

    /// remove all CRS (E)VLRs from the header
    pub fn remove_crs_vlrs(&mut self) {
        // remove projection vlrs
        self.vlrs = self.vlrs.drain(..).filter(|v| !v.is_projection()).collect();
        // remove projection evlrs
        self.evlrs = self
            .evlrs
            .drain(..)
            .filter(|v| !v.is_projection())
            .collect();
        self.has_wkt_crs = false;
    }

    /// Add a WKT CRS VLR to the header
    ///
    /// returns Err if the header already contains CRS (E)VLRs or the Las version is below 1.4
    ///
    /// The WKT bytes can be obtained from a horizontal EPSG code by using the crs_definitions crate
    pub fn set_wkt_crs(&mut self, wkt_string: impl Into<String>) -> Result<()> {
        if self.version() < crate::Version::new(1, 4) {
            return Err(Error::UnsupportedFeature {
                version: self.version(),
                feature: "WKT CRS VLR",
            });
        }

        for vlr in self.all_vlrs() {
            if vlr.is_projection() {
                return Err(Error::HeaderContainsCrsVlr)?;
            }
        }

        let mut user_id = [0; 16];
        for (i, c) in "LASF_Projection".as_bytes().iter().enumerate() {
            user_id[i] = *c;
        }

        let wkt_bytes = wkt_string.into().into_bytes();
        let num_bytes = wkt_bytes.len();

        if num_bytes > u16::MAX as usize {
            // must be added as an evlr, because of length
            let crs_vlr = crate::raw::Vlr {
                reserved: 0,
                user_id,
                record_id: 2112,
                record_length_after_header: crate::raw::vlr::RecordLength::Evlr(num_bytes as u64),
                description: [0; 32],
                data: wkt_bytes,
            };
            self.evlrs.push(crate::Vlr::new(crs_vlr));
        } else {
            // just a normal vlr
            let crs_vlr = crate::raw::Vlr {
                reserved: 0,
                user_id,
                record_id: 2112,
                record_length_after_header: crate::raw::vlr::RecordLength::Vlr(num_bytes as u16),
                description: [0; 32],
                data: wkt_bytes,
            };
            self.vlrs.push(crate::Vlr::new(crs_vlr));
        };

        self.has_wkt_crs = true;
        Ok(())
    }

    /// Get the WKT-CRS-data if the WKT-CRS (E)VLR exists
    pub fn get_wkt_crs(&self) -> Option<&[u8]> {
        for vlr in self.all_vlrs() {
            if let ("lasf_projection", 2112) = (vlr.user_id.to_lowercase().as_str(), vlr.record_id)
            {
                // lossy should be OK as only UTF8 chars should be present according to the las v1.4 specification
                return Some(vlr.data.as_slice());
            }
        }
        None
    }

    /// Get all the GeoTiff CRS data if the GeoTiff-CRS (E)VLR(s) exist
    pub fn get_geotiff_crs(&self) -> Result<Option<GeoTiffCrs>> {
        let mut geotiff_vlrs = [None, None, None];
        for vlr in self.all_vlrs() {
            if vlr.is_projection() {
                let pos = match vlr.record_id {
                    34735 => 0,
                    34736 => 1,
                    34737 => 2,
                    _ => continue,
                };

                geotiff_vlrs[pos] = Some(vlr.data.as_slice());
            }
        }
        let [geotiff_main, double, string] = geotiff_vlrs;

        if let Some(main_vlr) = geotiff_main {
            let mut main_vlr = Cursor::new(main_vlr);

            let _ = main_vlr.read_u16::<LittleEndian>()?; // should always 1
            let _ = main_vlr.read_u16::<LittleEndian>()?; // should always 1
            let _ = main_vlr.read_u16::<LittleEndian>()?; // should always 0
            let num_keys = main_vlr.read_u16::<LittleEndian>()?;

            let crs_data = GeoTiffCrs::read_from(main_vlr, double, string, num_keys)?;
            Ok(Some(crs_data))
        } else {
            Ok(None)
        }
    }
}

/// Get the EPSG code(s) from WKT-CRS bytes
///
/// Splits the wkt string in two at "VERT" and
/// finds the horizontal and vertical codes at the end of each substring
pub fn get_epsg_from_wkt_crs(bytes: &[u8]) -> Result<Option<EpsgCrs>> {
    let wkt = String::from_utf8_lossy(bytes);

    // VERT_CS for WKT v1 and VERTCRS for v2
    let pieces = if let Some((horizontal, vertical)) = wkt.split_once("VERT") {
        // both horizontal and vertical codes exist
        vec![horizontal.as_bytes(), vertical.as_bytes()]
    } else {
        // only horizontal code
        vec![wkt.as_bytes()]
    };

    let mut epsg = [None, None];
    for (pi, piece) in pieces.into_iter().enumerate() {
        let mut epsg_code = 0;
        let mut has_code_started = false;
        let mut power = 0;
        for (i, byte) in piece.iter().rev().enumerate() {
            if (48..=57).contains(byte) {
                // the byte is an ASCII encoded number
                has_code_started = true;

                epsg_code += 10_u16.pow(power) * (byte - 48) as u16;
                power += 1;
            } else if has_code_started {
                break;
            }
            if i > 7 {
                break;
            }
        }
        if epsg_code != 0 {
            epsg[pi] = Some(epsg_code);
        }
    }
    if epsg[0].is_none() {
        return Err(Error::UnreadableWktCrs);
    }

    Ok(Some(EpsgCrs {
        horizontal: epsg[0].unwrap(),
        vertical: epsg[1],
    }))
}

/// Get the EPSG code(s) from GeoTiff-CRS-data
pub fn get_epsg_from_geotiff_crs(geotiff_crs_data: GeoTiffCrs) -> Result<Option<EpsgCrs>> {
    let mut out = (None, None);
    for entry in geotiff_crs_data.entries {
        match entry.id {
            // 2048 and 3072 should not co-exist, but might both be combined with 4096
            // 1024 should always exist
            1024 => match entry.data {
                GeoTiffData::U16(0) => return Err(Error::UnreadableGeoTiffCrs),
                GeoTiffData::U16(1) => (), // projected crs
                GeoTiffData::U16(2) => (), // geographic crs
                GeoTiffData::U16(3) => (), // geographic + a vertical crs
                GeoTiffData::U16(32_767) => return Err(Error::UserDefinedCrs),
                p => return Err(Error::UnimplementedForGeoTiffStringAndDoubleData(p)),
            },
            2048 | 3072 => {
                if let GeoTiffData::U16(v) = entry.data {
                    out.0 = Some(v);
                } else {
                    // should probably add support for this
                    return Err(Error::UnimplementedForGeoTiffStringAndDoubleData(
                        entry.data,
                    ));
                }
            }
            4096 => {
                // vertical crs
                if let GeoTiffData::U16(v) = entry.data {
                    out.1 = Some(v);
                } else {
                    log!(
                        Level::Info,
                        "Unable to parse EPSG code from found vertical CRS component in GeoTiff data"
                    );
                }
            }
            _ => (), // the rest are descriptions and units.
        }
    }
    if out.0.is_none() {
        return Err(Error::UnreadableGeoTiffCrs);
    }
    Ok(Some(EpsgCrs {
        horizontal: out.0.unwrap(),
        vertical: out.1,
    }))
}

/// Struct for the GeoTiff CRS data
#[derive(Debug)]
pub struct GeoTiffCrs {
    /// The GeoTiff key entries
    pub entries: Vec<GeoTiffKeyEntry>,
}

impl GeoTiffCrs {
    fn read_from(
        mut main_vlr: Cursor<&[u8]>,
        double_vlr: Option<&[u8]>,
        ascii_vlr: Option<&[u8]>,
        count: u16,
    ) -> Result<Self> {
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entries.push(GeoTiffKeyEntry::read_from(
                &mut main_vlr,
                &double_vlr,
                &ascii_vlr,
            )?);
        }
        Ok(GeoTiffCrs { entries })
    }
}

/// GeoTiff data enum
/// GeoTiff data can either be a u16, an ascii string or sequence of f64
#[derive(Debug)]
pub enum GeoTiffData {
    /// a single u16
    U16(u16),
    /// an ascii string
    String(String),
    /// a sequence of f64
    Doubles(Vec<f64>),
}

/// A single GeoTiff key entry
#[derive(Debug)]
pub struct GeoTiffKeyEntry {
    /// The Id of the entry
    pub id: u16,
    /// The data in this entry
    pub data: GeoTiffData,
}

impl GeoTiffKeyEntry {
    fn read_from(
        main_vlr: &mut Cursor<&[u8]>,
        double_vlr: &Option<&[u8]>,
        ascii_vlr: &Option<&[u8]>,
    ) -> Result<Self> {
        let id = main_vlr.read_u16::<LittleEndian>()?;
        let location = main_vlr.read_u16::<LittleEndian>()?;
        let count = main_vlr.read_u16::<LittleEndian>()?;
        let offset = main_vlr.read_u16::<LittleEndian>()?;
        let data = match location {
            0 => GeoTiffData::U16(offset),
            34736 => {
                let mut cursor = Cursor::new(double_vlr.ok_or(Error::UnreadableGeoTiffCrs)?);
                let _ = cursor.seek(SeekFrom::Start(offset as u64 * 8_u64))?; // 8 is the byte size of a f64 and offset is not a byte offset but an index
                let mut doubles = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    doubles.push(cursor.read_f64::<LittleEndian>()?);
                }
                GeoTiffData::Doubles(doubles)
            }
            34737 => {
                let mut cursor = Cursor::new(ascii_vlr.ok_or(Error::UnreadableGeoTiffCrs)?);
                let _ = cursor.seek(SeekFrom::Start(offset as u64))?; // no need to multiply the index as the byte size of char is 1
                let mut string = String::with_capacity(count as usize);
                for _ in 0..count {
                    string.push(cursor.read_u8()? as char);
                }
                GeoTiffData::String(string)
            }
            _ => return Err(Error::UndefinedDataForGeoTiffKey(id)),
        };
        Ok(GeoTiffKeyEntry { id, data })
    }
}

#[cfg(test)]
mod tests {
    use crate::Reader;

    #[cfg(feature = "laz")]
    #[test]
    fn test_get_epsg_crs_wkt_vlr_autzen() {
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let crs = reader.header().get_epsg_crs().unwrap().unwrap();
        assert!(crs.horizontal == 2992);
        assert!(crs.vertical == Some(6360))
    }

    #[cfg(feature = "laz")]
    #[test]
    fn test_get_epsg_crs_geotiff_vlr_norway() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let crs = reader.header().get_epsg_crs().unwrap().unwrap();
        assert!(crs.horizontal == 25832);
        assert!(crs.vertical == Some(5941));
    }

    #[cfg(feature = "laz")]
    #[test]
    fn test_remove_crs_vlrs() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        header.remove_crs_vlrs();

        for vlr in header.all_vlrs() {
            if vlr.is_projection() {
                panic!("CRS VLRs are still in the header")
            }
        }
    }

    #[cfg(feature = "laz")]
    #[test]
    fn test_write_crs_vlr_las_v1_4() {
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        // remove the current crs vlr(s)
        header.remove_crs_vlrs();

        // add a new crs vlr (not the correct one, but does not matter)
        header
            .set_wkt_crs(crs_definitions::from_code(3006).unwrap().wkt.as_bytes())
            .expect("Could not add wkt crs vlr");

        let crs = header.get_epsg_crs().expect("Could not parse crs").unwrap();

        assert!(crs.horizontal == 3006);
        assert!(crs.vertical.is_none());
    }

    #[test]
    fn test_write_crs_vlr_las_v1_2() {
        let reader = Reader::from_path("tests/data/autzen.las").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        // remove the current crs vlr(s)
        header.remove_crs_vlrs();

        // try to add a new crs vlr (not supported for las 1.4)
        let res = header.set_wkt_crs(crs_definitions::from_code(3006).unwrap().wkt);

        assert!(res.is_err());
    }
}
