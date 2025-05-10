use byteorder::{LittleEndian, ReadBytesExt};
use log::{log, Level};
use thiserror::Error;

use std::io::{Cursor, Seek, SeekFrom};

use crate::{Error, Header, Result};

/// horizontal and optional vertical crs given by EPSG codes
#[derive(Debug, Clone, Copy)]
pub struct EpsgCrs {
    pub horizontal: u16,
    pub vertical: Option<u16>,
}

impl Header {
    /// For parsing CRS's from a header
    /// Las stores CRS-info in (E)VLRs
    /// parsing from (E)VLR(s) with WKT-CRS v1 or v2 or GeoTiff U16-data is supported
    ///
    /// The CRS is returend in a Result<Crs, CrsError>
    /// CRS has the fields horizontal, which is a u16 EPSG code, and vertical, which is an optional u16 EPSG code.
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
    ///
    /// # Example
    ///
    /// ```
    /// use las::Reader;
    /// let reader = Reader::from_path("lidar.las").expect("Cannot open reader");
    /// let crs = reader.header().parse_crs().expect("Cannot parse CRS-VLR");
    /// ```
    pub fn parse_crs(&self) -> Result<Option<EpsgCrs>> {
        let mut crs_vlrs = [None, None, None, None];
        for vlr in self.all_vlrs() {
            if vlr.is_projection() {
                let pos = match vlr.record_id {
                    2112 => 0,
                    34735 => 1,
                    34736 => 2,
                    34737 => 3,
                    _ => unreachable!(),
                };

                crs_vlrs[pos] = Some(vlr.data.as_slice());
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

        let crs = if let Some(wkt) = wkt {
            get_wkt_epsg(wkt)?
        } else if let Some(main) = geotiff_main {
            get_geotiff_epsg(main, double, string)?
        } else {
            None
        };
        Ok(crs)
    }

    /// remove all CRS (E)VLRs from the header
    pub fn remove_crs_vlrs(&mut self) {
        // check vlrs
        let mut crs_vlr_indecies = vec![];
        for (i, vlr) in self.vlrs.iter().enumerate() {
            if vlr.is_projection() {
                crs_vlr_indecies.push(i);
            }
        }
        crs_vlr_indecies.sort_by(|a, b| b.cmp(a));

        // preserves the order of the rest of the vlrs
        for index in crs_vlr_indecies {
            let _ = self.vlrs.remove(index);
        }

        // check evlrs
        let mut crs_evlr_indecies = vec![];
        for (i, vlr) in self.evlrs.iter().enumerate() {
            if vlr.is_projection() {
                crs_evlr_indecies.push(i);
            }
        }
        crs_evlr_indecies.sort_by(|a, b| b.cmp(a));

        // preserves the order of the rest of the evlrs
        for index in crs_evlr_indecies {
            let _ = self.evlrs.remove(index);
        }

        self.has_wkt_crs = false;
    }

    /// Add a WKT CRS VLR to the header
    ///
    /// returns Err if the header already contains CRS VLRs or the las version is below 1.4
    ///
    /// The WKT bytes can be obtained from a horizontal EPSG code by using the crs_definitions crate
    pub fn set_wkt_crs(&mut self, wkt_bytes: &[u8]) -> Result<()> {
        if self.version().minor < 4 {
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

        let crs_vlr = crate::raw::Vlr {
            reserved: 0,
            user_id,
            record_id: 2112,
            record_length_after_header: crate::raw::vlr::RecordLength::Vlr(wkt_bytes.len() as u16),
            description: [0; 32],
            data: wkt_bytes.to_vec(),
        };
        self.vlrs.push(crate::Vlr::new(crs_vlr));

        self.has_wkt_crs = true;
        Ok(())
    }
}

/// find the EPSG codes for the WKT string
///
/// split the wkt string in two at VERTCRS
/// and find the horizontal and vertical codes at the end of each substring
fn get_wkt_epsg(bytes: &[u8]) -> Result<Option<EpsgCrs>> {
    let wkt: String = bytes.iter().map(|b| *b as char).collect();

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

/// Gets the EPSG codes from the geotiff crs vlrs
fn get_geotiff_epsg(
    main_vlr: &[u8],
    double_vlr: Option<&[u8]>,
    ascii_vlr: Option<&[u8]>,
) -> Result<Option<EpsgCrs>> {
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
                GeoTiffData::U16(0) => return Err(Error::UnreadableGeoTiffCrs),
                GeoTiffData::U16(1) => (), // projected crs
                GeoTiffData::U16(2) => (), // geographic coordinates
                GeoTiffData::U16(3) => (), // geographic + a vertical crs
                GeoTiffData::U16(32_767) => return Err(Error::UserDefinedCrs),
                p => return Err(Error::UnimplementedForGeoTiffStringAndDoubleData(p)),
            },
            2048 | 3072 => {
                if let GeoTiffData::U16(v) = entry.data {
                    out.0 = Some(v);
                } else {
                    // should probably add support for this
                    return Err(Error::UndefinedDataForGeoTiffKey(entry.id));
                }
            }
            4096 => {
                // vertical crs
                if let GeoTiffData::U16(v) = entry.data {
                    out.1 = Some(v);
                } else {
                    // should probably add support for this
                    return Err(Error::UndefinedDataForGeoTiffKey(4096));
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

#[derive(Debug)]
struct GeoTiffCRS {
    entries: Vec<GeoTiffKeyEntry>,
}

impl GeoTiffCRS {
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

    #[test]
    fn test_parse_crs_wkt_vlr_autzen() {
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let crs = reader.header().parse_crs().unwrap().unwrap();
        assert!(crs.horizontal == 2992);
        assert!(crs.vertical == Some(6360))
    }

    #[test]
    fn test_parse_crs_geotiff_vlr_norway() {
        let reader =
            Reader::from_path("tests/data/32-1-472-150-76.laz").expect("Cannot open reader");
        let crs = reader.header().parse_crs().unwrap().unwrap();
        assert!(crs.horizontal == 25832);
        assert!(crs.vertical == Some(5941));
    }

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

    #[test]
    fn test_write_crs_vlr() {
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let mut header = reader.header().to_owned();
        // remove the current crs vlr(s)
        header.remove_crs_vlrs();

        // add a new crs vlr (not the correct one, but does not matter)
        header
            .set_wkt_crs(crs_definitions::from_code(3006).unwrap().wkt().as_bytes())
            .expect("Could not add wkt crs vlr");

        let crs = header.parse_crs().expect("Could not parse crs").unwrap();

        assert!(crs.horizontal == 3006);
        assert!(crs.vertical.is_none());
    }
}
