/// For parsing CRS's stored in WKT-CRS v1 and v2 and GeoTiff U16 (E)VLR(s) in the Header
///
/// The CRS is returend in a Result<Crs, CrsError>
/// CRS has the fields horizontal, which is a u16 EPSG code, and vertical, which is an optional u16 EPSG code.
/// Only horizontal CRS's are detected for WKT-CRS (E)VLRs
/// Geotiff-CRS (E)VLRs might have both
///
/// The validity of the extracted code is not checked.
/// Use the crs-definitions crate for checking validity of EPSG codes.
///
/// Be aware that certain software adds invalid CRS VLRs when writing CRS-less lidar files (f.ex when QGIS convert .la[s,z] files without a CRS-VLR to .copc.laz files).
/// This is because the las 1.4 spec (which .copc.laz demands), requires a WKT-CRS (E)VLR to be present.
/// These VLRs often contain the invalid EPSG code 0.
///
/// Userdefined CRS's and CRS's stored in GeoTiff string or Doubles data is not yet supported.
/// The different Error's are described in the CrsError enum
use byteorder::{LittleEndian, ReadBytesExt};
use log::{log, Level};
use thiserror::Error;

use std::io::{Cursor, Seek, SeekFrom};

use super::Header;

/// crs result
type CrsResult<T> = Result<T, CrsError>;

/// horizontal and optional vertical crs given by EPSG code
#[derive(Debug, Clone, Copy)]
pub struct Crs {
    pub horizontal: u16,
    pub vertical: Option<u16>,
}

/// Crs-specific error enum
#[derive(Error, Debug)]
pub enum CrsError {
    #[error("The header does not contain any CRS VLRs")]
    NoCrs,
    #[error("Parsing of User Defined CRS not implemented")]
    UserDefinedCrs,
    #[error("Unable to parse the found WKT-CRS (E)VLR")]
    UnreadableWktCrs,
    #[error("Unable to parse the found Geotiff (E)VLR(s)")]
    UnreadableGeotiffCrs,
    #[error("Invalid key for Geotiff data")]
    UndefinedDataForGeoTiffKey(u16),
    #[error("The CRS parser does not handle CRS's defined by Geotiff String and Double data")]
    UnimplementedForGeoTiffStringAndDoubleData(GeoTiffData),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Header {
    /// Extracts the CRS by EPSG code stored in the CRS (E)VLR(s), if they exist
    /// Supports both WKT definitions (by finding the EPSG code on the end of the WKT-string) and
    /// legacy GeoTiff (but not GeoTiff string and double defined CRSs, only u16 EPSG code)
    /// but most CRSs should be detected
    pub fn parse_crs(&self) -> CrsResult<Crs> {
        let mut crs_vlrs = [None, None, None, None];
        for vlr in self.all_vlrs() {
            if let ("lasf_projection", 2112 | 34735 | 34736 | 34737) =
                (vlr.user_id.to_lowercase().as_str(), vlr.record_id)
            {
                let pos = match vlr.record_id {
                    2112 => 0,
                    34735 => 1,
                    34736 => 2,
                    34737 => 3,
                    _ => unreachable!(),
                };

                crs_vlrs[pos] = Some(&vlr.data);
            }
        }
        let [wkt, geotiff_main, double, string] = crs_vlrs;

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

        // warn about double defined CRS
        if wkt.is_some() && geotiff_main.is_some() {
            log!(
                Level::Warn,
                "Both WKT and Geotiff CRS (E)VLRs found, WKT is parsed"
            );
        }

        if let Some(wkt) = wkt {
            get_wkt_epsg(wkt)
        } else if let Some(main) = geotiff_main {
            get_geotiff_epsg(main, double, string)
        } else {
            Err(CrsError::NoCrs)
        }
    }
}

/// find the epsg code located at the end of the WKT string
fn get_wkt_epsg(bytes: &[u8]) -> CrsResult<Crs> {
    let mut epsg_code = 0;
    let mut has_code_started = false;
    let mut power = 0;
    for (i, byte) in bytes.iter().rev().enumerate() {
        if (48..=57).contains(byte) {
            // the byte is an ASCII encoded number
            has_code_started = true;

            epsg_code += 10_u16.pow(power) * (byte - 48) as u16;
            power += 1;
        } else if has_code_started {
            break;
        }
        if i > 7 {
            // the code should be a 4 or 5 digit number starting at index 2 or 3 from behind
            // meaning that if i has reached 8 something is wrong
            return Err(CrsError::UnreadableWktCrs);
        }
    }
    Ok(Crs {
        horizontal: epsg_code,
        vertical: None,
    })
}

/// Gets the EPSG code in the geotiff crs vlrs
/// returns a tuple containing the horizontal code and the optional vertical code
fn get_geotiff_epsg(
    main_vlr: &[u8],
    double_vlr: Option<&Vec<u8>>,
    ascii_vlr: Option<&Vec<u8>>,
) -> CrsResult<Crs> {
    let mut main_vlr = Cursor::new(main_vlr);

    let _ = main_vlr.read_u16::<LittleEndian>()?; // always 1
    let _ = main_vlr.read_u16::<LittleEndian>()?; // always 1
    let _ = main_vlr.read_u16::<LittleEndian>()?; // always 0
    let num_keys = main_vlr.read_u16::<LittleEndian>()?;

    let crs_data = GeoTiffCRS::read_from(main_vlr, double_vlr, ascii_vlr, num_keys)?;

    let mut out = (None, None);
    for entry in crs_data.entries {
        match entry.id {
            // 3072 and 2048 should not co-exist, but might both be combined with 4096
            // 1024 should always exist
            1024 => match entry.data {
                GeoTiffData::U16(0) => return Err(CrsError::UnreadableGeotiffCrs),
                GeoTiffData::U16(1) => (), // projected crs
                GeoTiffData::U16(2) => (), // geographic coordinates
                GeoTiffData::U16(3) => (), // geographic + a vertical crs
                GeoTiffData::U16(32_767) => return Err(CrsError::UserDefinedCrs),
                p => return Err(CrsError::UnimplementedForGeoTiffStringAndDoubleData(p)),
            },
            2048 | 3072 => {
                if let GeoTiffData::U16(v) = entry.data {
                    out.0 = Some(v);
                } else {
                    // should probably add support for this
                    return Err(CrsError::UndefinedDataForGeoTiffKey(entry.id));
                }
            }
            4096 => {
                // vertical crs
                if let GeoTiffData::U16(v) = entry.data {
                    out.1 = Some(v);
                } else {
                    // should probably add support for this
                    return Err(CrsError::UndefinedDataForGeoTiffKey(4096));
                }
            }
            _ => (), // the rest are descriptions and units.
        }
    }
    if out.0.is_none() {
        return Err(CrsError::UnreadableGeotiffCrs);
    }
    Ok(Crs {
        horizontal: out.0.unwrap(),
        vertical: out.1,
    })
}

#[derive(Debug)]
struct GeoTiffCRS {
    entries: Vec<GeoTiffKeyEntry>,
}

impl GeoTiffCRS {
    fn read_from(
        mut main_vlr: Cursor<&[u8]>,
        double_vlr: Option<&Vec<u8>>,
        ascii_vlr: Option<&Vec<u8>>,
        count: u16,
    ) -> CrsResult<Self> {
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entries.push(GeoTiffKeyEntry::read_from(
                &mut main_vlr,
                &double_vlr,
                &ascii_vlr,
            )?);
        }
        Ok(GeoTiffCRS { entries })
    }
}

#[derive(Debug)]
pub enum GeoTiffData {
    U16(u16),
    String(String),
    Doubles(Vec<f64>),
}

#[derive(Debug)]
struct GeoTiffKeyEntry {
    id: u16,
    data: GeoTiffData,
}

impl GeoTiffKeyEntry {
    fn read_from(
        main_vlr: &mut Cursor<&[u8]>,
        double_vlr: &Option<&Vec<u8>>,
        ascii_vlr: &Option<&Vec<u8>>,
    ) -> CrsResult<Self> {
        let id = main_vlr.read_u16::<LittleEndian>()?;
        let location = main_vlr.read_u16::<LittleEndian>()?;
        let count = main_vlr.read_u16::<LittleEndian>()?;
        let offset = main_vlr.read_u16::<LittleEndian>()?;
        let data = match location {
            0 => GeoTiffData::U16(offset),
            34736 => {
                let mut cursor = Cursor::new(double_vlr.ok_or(CrsError::UnreadableGeotiffCrs)?);
                let _ = cursor.seek(SeekFrom::Start(offset as u64 * 8_u64))?; // 8 is the byte size of a f64 and offset is not a byte offset but an index
                let mut doubles = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    doubles.push(cursor.read_f64::<LittleEndian>()?);
                }
                GeoTiffData::Doubles(doubles)
            }
            34737 => {
                let mut cursor = Cursor::new(ascii_vlr.ok_or(CrsError::UnreadableGeotiffCrs)?);
                let _ = cursor.seek(SeekFrom::Start(offset as u64))?; // no need to multiply the index as the byte size of char is 1
                let mut string = String::with_capacity(count as usize);
                for _ in 0..count {
                    string.push(cursor.read_u8()? as char);
                }
                GeoTiffData::String(string)
            }
            _ => return Err(CrsError::UndefinedDataForGeoTiffKey(id)),
        };
        Ok(GeoTiffKeyEntry { id, data })
    }
}

#[cfg(test)]
mod tests {
    use crate::Reader;

    #[test]
    fn test_parse_crs_wkt_vlr_autzen() {
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let crs = reader.header().parse_crs().unwrap();
        assert!(crs.horizontal == 6360);
        assert!(crs.vertical.is_none())
    }

    #[test]
    fn test_parse_crs_geotiff_vlr_norway() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let crs = reader.header().parse_crs().unwrap();
        assert!(crs.horizontal == 25832);
        assert!(crs.vertical == Some(5941));
    }
}
