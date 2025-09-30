//! Module for handling Coordinate Reference System (CRS) data in a headers variable length records
//!
//! CRSes are stored either as [WKT](https://en.wikipedia.org/wiki/Well-known_text_representation_of_geometry) or as [GeoTiff tags](https://docs.ogc.org/is/19-008r4/19-008r4.html).
//! [Header::get_epsg_crs] parses the CRS data to [EPSG](https://en.wikipedia.org/wiki/EPSG_Geodetic_Parameter_Dataset) code(s).
//!
//! Only WKT is supported for writing CRS data to a header.

use crate::{Error, Header, Result, Vlr};
use byteorder::{LittleEndian, ReadBytesExt};
use log::{log, Level};
use std::io::{Cursor, Seek, SeekFrom};

const EPSG_RANGE: std::ops::RangeInclusive<u16> = 1024..=(i16::MAX as u16);

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
    ///
    /// Las stores CRS-info in (E)VLRs either as Well Known Text (WKT) or in GeoTIff-format
    /// Most (not all!) CRSes used for Aerial Lidar has an associated EPSG code.
    /// Use this function to try and parse the EPSG code(s) from the VLR data.
    ///
    /// WKT takes precedence over GeoTiff in this function, but they should not co-exist.
    ///
    /// Just because this function fails does not mean that no CRS-data is available.
    /// Use functions [Self::get_wkt_crs_bytes] or [Self::get_geotiff_crs] to get all data stored in the CRS-(E)VLRs.
    ///
    /// Parsing code(s) from WKT-CRS v1 or v2 or GeoTiff U16-data is supported.
    ///
    /// The validity of the extracted code is not checked.
    /// Use the [crs-definitions](https://docs.rs/crs-definitions/latest/crs_definitions/) crate for checking the validity of a horizontal EPSG code.
    ///
    /// # Example
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("tests/data/autzen.las").expect("Cannot open reader");
    /// let crs = reader.header().get_epsg_crs().expect("Cannot parse EPSG code(s) from the CRS-(E)VLRs");
    /// ```
    pub fn get_epsg_crs(&self) -> Result<Option<EpsgCrs>> {
        if let Some(wkt) = self.get_wkt_crs_bytes() {
            if !self.has_wkt_crs() {
                log!(
                    Level::Warn,
                    "WKT CRS (E)VLR found, but header says it does not exist"
                );
            }
            Ok(Some(get_epsg_from_wkt_crs_bytes(wkt)?))
        } else if let Some(geotiff) = self.get_geotiff_crs()? {
            if self.has_wkt_crs() {
                log!(
                    Level::Warn,
                    "Only Geotiff CRS (E)VLRs found, but header says WKT exists"
                );
            }
            Ok(Some(get_epsg_from_geotiff_crs(geotiff)?))
        } else {
            if self.has_wkt_crs() {
                log!(
                    Level::Warn,
                    "No WKT CRS (E)VLR found, but header says it exists"
                );
            }
            Ok(None)
        }
    }

    /// Removes all CRS (E)VLRs from the header
    pub fn remove_crs_vlrs(&mut self) {
        self.vlrs = self.vlrs.drain(..).filter(|v| !v.is_projection()).collect();
        self.evlrs = self
            .evlrs
            .drain(..)
            .filter(|v| !v.is_projection())
            .collect();
        self.has_wkt_crs = false;
    }

    /// Adds a WKT CRS VLR to the header
    ///
    /// Returns Err if the header already contains CRS (E)VLRs or the Las version is below 1.4.
    ///
    /// The WKT bytes can be obtained from a horizontal EPSG code by using the [crs-definitions](https://docs.rs/crs-definitions/latest/crs_definitions/) crate
    pub fn set_wkt_crs(&mut self, wkt_crs_bytes: Vec<u8>) -> Result<()> {
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

        let num_bytes = wkt_crs_bytes.len();
        let vlr = Vlr {
            user_id: "LASF_Projection".to_string(),
            record_id: 2112,
            description: String::new(),
            data: wkt_crs_bytes,
        };
        if num_bytes > u16::MAX as usize {
            self.evlrs.push(vlr);
        } else {
            self.vlrs.push(vlr);
        };
        self.has_wkt_crs = true;

        Ok(())
    }

    /// Gets the WKT-CRS-data if the WKT-CRS (E)VLR exists
    pub fn get_wkt_crs_bytes(&self) -> Option<&[u8]> {
        for vlr in self.all_vlrs() {
            if vlr.is_crs_wkt() {
                return Some(vlr.data.as_slice());
            }
        }
        None
    }

    /// Gets all the GeoTiff CRS data if the GeoTiff-CRS (E)VLR(s) exist
    pub fn get_geotiff_crs(&self) -> Result<Option<GeoTiffCrs>> {
        let mut main_vlr = None;
        let mut double_vlr = None;
        let mut ascii_vlr = None;
        for vlr in self.all_vlrs() {
            if vlr.is_projection() {
                match vlr.record_id {
                    34735 => {
                        main_vlr = Some(vlr.data.as_slice());
                    }
                    34736 => {
                        double_vlr = Some(vlr.data.as_slice());
                    }
                    34737 => {
                        ascii_vlr = Some(vlr.data.as_slice());
                    }
                    _ => continue,
                };
            }
        }
        if let Some(main_vlr) = main_vlr {
            Ok(Some(GeoTiffCrs::read_from(
                main_vlr, double_vlr, ascii_vlr,
            )?))
        } else {
            Ok(None)
        }
    }
}

/// Tries to parse EPSG code(s) from WKT-CRS bytes.
///
/// By parsing the EPSG codes at the end of the vertical and horizontal CRS sub-strings
/// This is not true WKT parser and might provide a bad code if
/// the WKT-CRS bytes does not look as expected
pub fn get_epsg_from_wkt_crs_bytes(bytes: &[u8]) -> Result<EpsgCrs> {
    let wkt = String::from_utf8_lossy(bytes);

    // VERT_CS for WKT v1 and VERTCRS or VERTICALCRS for v2
    let pieces = if let Some((horizontal, vertical)) = wkt.split_once("VERTCRS") {
        vec![horizontal.as_bytes(), vertical.as_bytes()]
    } else if let Some((horizontal, vertical)) = wkt.split_once("VERTICALCRS") {
        vec![horizontal.as_bytes(), vertical.as_bytes()]
    } else if let Some((horizontal, vertical)) = wkt.split_once("VERT_CS") {
        vec![horizontal.as_bytes(), vertical.as_bytes()]
    } else {
        vec![wkt.as_bytes()]
    };

    let mut epsg = [None, None];
    for (pi, piece) in pieces.into_iter().enumerate() {
        // the EPSG code is located at the end of the substrings
        // and so we iterate through the substrings backwards collecting
        // digits and adding them to our EPSG code
        let mut epsg_code = 0;
        let mut code_has_started = false;
        let mut power = 1;
        // the 10 last bytes should be enough (with a small margin)
        // as the code is 4 or 5 digits starting at the 2 or 3 byte from the back
        for byte in piece.iter().rev().take(10) {
            // if the byte is an ASCII encoded digit
            if (48..=57).contains(byte) {
                // mark that the EPSG code has started
                // so that we can break when we no
                // longer find digits
                code_has_started = true;

                // translate from ASCII to digits
                // and multiply by powers of 10
                // sum it to build the EPSG
                // code digit by digit
                epsg_code += power * (byte - 48) as u16;
                power *= 10;
            } else if code_has_started {
                // we no longer see digits
                // so the code must be over
                break;
            }
        }
        if EPSG_RANGE.contains(&epsg_code) {
            epsg[pi] = Some(epsg_code);
        }
    }
    if let Some(horizontal) = epsg[0] {
        Ok(EpsgCrs {
            horizontal,
            vertical: epsg[1],
        })
    } else {
        Err(Error::UnreadableWktCrs)
    }
}

/// Get the EPSG code(s) from GeoTiff-CRS-data
pub fn get_epsg_from_geotiff_crs(geotiff_crs_data: GeoTiffCrs) -> Result<EpsgCrs> {
    let mut out = (0, None);
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
                    out.0 = v;
                }
            }
            4096 => {
                // vertical crs
                if let GeoTiffData::U16(v) = entry.data {
                    out.1 = Some(v);
                }
            }
            _ => (), // the rest are descriptions and units.
        }
    }
    if !EPSG_RANGE.contains(&out.0) {
        return Err(Error::UnreadableGeoTiffCrs);
    }
    Ok(EpsgCrs {
        horizontal: out.0,
        vertical: out.1,
    })
}

/// Struct for the GeoTiff CRS data
#[derive(Debug)]
pub struct GeoTiffCrs {
    /// The GeoTiff key entries
    pub entries: Vec<GeoTiffKeyEntry>,
}

impl GeoTiffCrs {
    fn read_from(
        main_vlr: &[u8],
        double_vlr: Option<&[u8]>,
        ascii_vlr: Option<&[u8]>,
    ) -> Result<Self> {
        let mut main_vlr = Cursor::new(main_vlr);

        let _ = main_vlr.read_u16::<LittleEndian>()?; // should always be 1
        let _ = main_vlr.read_u16::<LittleEndian>()?; // should always be 1
        let _ = main_vlr.read_u16::<LittleEndian>()?; // should always be 0
        let num_keys = main_vlr.read_u16::<LittleEndian>()?;

        let mut entries = Vec::with_capacity(num_keys as usize);
        for _ in 0..num_keys {
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
            .set_wkt_crs(
                crs_definitions::from_code(3006)
                    .unwrap()
                    .wkt
                    .as_bytes()
                    .to_vec(),
            )
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
        let res = header.set_wkt_crs(
            crs_definitions::from_code(3006)
                .unwrap()
                .wkt
                .as_bytes()
                .to_vec(),
        );

        assert!(res.is_err());
    }
}
