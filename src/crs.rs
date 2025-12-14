//! Module for handling Coordinate Reference System (CRS) data in a headers variable length records
//!
//! CRSes are stored either as [WKT](https://en.wikipedia.org/wiki/Well-known_text_representation_of_geometry) or as [GeoTiff tags](https://docs.ogc.org/is/19-008r4/19-008r4.html).
//! Use [Header::get_wkt_crs_bytes] or [Header::get_geotiff_crs] respectively to read the crs-data from the header's (E)VLRs.
//! The returned objects are not CRS-aware, they have only parsed the data available in the CRS-(E)VLRs.
//! Use the [las-crs](https://docs.rs/las-crs/latest/las_crs) crate to parse the data to EPSG codes.
//!
//! Only WKT is supported for writing CRS data to a header and only for las version 1.4.

use crate::{Error, Header, Result, Vlr};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Seek, SeekFrom};

impl Header {
    /// Removes all CRS (E)VLRs from the header
    pub fn remove_crs_vlrs(&mut self) {
        self.vlrs.retain(|v| !v.is_crs());
        self.evlrs.retain(|v| !v.is_crs());
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

        if self.all_vlrs().any(|v| v.is_crs()) {
            return Err(Error::HeaderContainsCrsVlr);
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
        self.all_vlrs()
            .find(|&v| v.is_wkt_crs())
            .map(|cv| cv.data.as_slice())
    }

    /// Gets all the GeoTiff CRS data if the GeoTiff-CRS (E)VLR(s) exist
    pub fn get_geotiff_crs(&self) -> Result<Option<GeoTiffCrs>> {
        let mut main_vlr = None;
        let mut double_vlr = None;
        let mut ascii_vlr = None;
        for vlr in self.all_vlrs().filter(|&v| v.is_geotiff_crs()) {
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
        if let Some(main_vlr) = main_vlr {
            Ok(Some(GeoTiffCrs::read_from(
                main_vlr, double_vlr, ascii_vlr,
            )?))
        } else {
            Ok(None)
        }
    }
}

/// Struct for the GeoTiff CRS data
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum GeoTiffData {
    /// a single u16
    U16(u16),
    /// an ascii string
    String(String),
    /// a sequence of f64
    Doubles(Vec<f64>),
}

/// A single GeoTiff key entry
#[derive(Debug, Clone)]
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
        let crs = reader
            .header()
            .get_wkt_crs_bytes()
            .expect("Could not get WKT bytes");

        let crs_str = String::from_utf8_lossy(crs);
        let (horizontal_component, vertical_component) = crs_str.split_once("VERT_CS").unwrap();

        // NAD83 / Oregon GIC Lambert (ft)
        let horizontal_crs = "AUTHORITY[\"EPSG\",\"2992\"]";
        assert!(horizontal_component.contains(horizontal_crs));

        // NAVD88 height (ftUS)
        let vertical_crs = "AUTHORITY[\"EPSG\",\"6360\"]";
        assert!(vertical_component.contains(vertical_crs));
    }

    #[cfg(feature = "laz")]
    #[test]
    fn test_get_epsg_crs_geotiff_vlr_norway() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let crs = reader.header().get_geotiff_crs().unwrap().unwrap();

        let horizontal = crs
            .entries
            .iter()
            .find(|key| key.id == 2048 || key.id == 3072)
            .unwrap()
            .data
            .clone();
        let vertical = crs
            .entries
            .iter()
            .find(|key| key.id == 4096)
            .unwrap()
            .data
            .clone();

        if let crate::crs::GeoTiffData::U16(h_code) = horizontal {
            assert!(h_code == 25832);
        } else {
            panic!("Expected GeoTiffData::U16")
        }
        if let crate::crs::GeoTiffData::U16(v_code) = vertical {
            assert!(v_code == 5941);
        } else {
            panic!("Expected GeoTiffData::U16")
        }
    }

    #[cfg(feature = "laz")]
    #[test]
    fn test_remove_crs_vlrs() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        header.remove_crs_vlrs();

        for vlr in header.all_vlrs() {
            if vlr.is_crs() {
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

        let random_bytes =
            "Test bytes. Just seeing if writing and reading is consitent:)".as_bytes();

        // add a new crs vlr (not the correct one, but does not matter)
        header
            .set_wkt_crs(random_bytes.to_vec())
            .expect("Could not add wkt crs vlr");

        let read_bytes = header.get_wkt_crs_bytes().unwrap();

        assert!(read_bytes == random_bytes);
    }

    #[test]
    fn test_write_crs_vlr_las_v1_2() {
        let reader = Reader::from_path("tests/data/autzen.las").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        // remove the current crs vlr(s)
        header.remove_crs_vlrs();

        // try to add a new crs vlr (not supported below las 1.4)
        let res = header.set_wkt_crs("just some bytes".as_bytes().to_vec());

        assert!(res.is_err());
    }
}
