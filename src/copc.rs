//! [COPC](https://copc.io/) header data

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    collections::HashMap,
    io::{Read, Write},
};
/// The user id of the LasZip VLR header.
pub const USER_ID: &str = "copc";
/// The description of the LasZip VLR header.
pub const DESCRIPTION: &str = "https://copc.io";

use crate::{Error, Header, Result, Vlr};
/// The COPC Info Vlr
///
/// The info VLR MUST exist.
/// The info VLR MUST be the first VLR in the file (must begin at offset 375
/// from the beginning of the file).
/// The info VLR is 160 bytes described by the following structure. reserved
/// elements MUST be set to 0.
#[derive(Debug)]
pub struct CopcInfoVlr {
    // Actual (unscaled) X coordinate of center of octree
    center_x: f64,
    // Actual (unscaled) Y coordinate of center of octree
    center_y: f64,
    // Actual (unscaled) Z coordinate of center of octree
    center_z: f64,
    // Perpendicular distance from the center to any side of the root node.
    halfsize: f64,
    // Space between points at the root node.
    // This value is halved at each octree level
    spacing: f64,
    // File offset to the first hierarchy page
    root_hier_offset: u64,
    // Size of the first hierarchy page in bytes
    root_hier_size: u64,
    // Minimum of GPSTime
    gpstime_minimum: f64,
    // Maximum of GPSTime
    gpstime_maximum: f64,
    // Must be 0
    reserved: [u64; 11],
}
impl CopcInfoVlr {
    /// The record id of the LasZip VLR header.
    pub const RECORD_ID: u16 = 1;

    /// Reads the Vlr data from the source.
    ///
    /// This **only** reads the *payload data* the
    /// vlr header should already be read.
    fn read_from<R: Read>(mut src: R) -> Result<Self> {
        Ok(Self {
            center_x: src.read_f64::<LittleEndian>()?,
            center_y: src.read_f64::<LittleEndian>()?,
            center_z: src.read_f64::<LittleEndian>()?,
            halfsize: src.read_f64::<LittleEndian>()?,
            spacing: src.read_f64::<LittleEndian>()?,
            root_hier_offset: src.read_u64::<LittleEndian>()?,
            root_hier_size: src.read_u64::<LittleEndian>()?,
            gpstime_minimum: src.read_f64::<LittleEndian>()?,
            gpstime_maximum: src.read_f64::<LittleEndian>()?,
            reserved: {
                let mut reserved = [0; 11];
                for field in reserved.iter_mut() {
                    *field = src.read_u64::<LittleEndian>()?;
                }
                reserved
            },
        })
    }

    /// Writes the Vlr data to the source.
    ///
    /// This **only** writes the *payload data* the
    /// vlr header should be written before-hand.
    pub fn write_to<W: Write>(&self, dst: &mut W) -> Result<()> {
        dst.write_f64::<LittleEndian>(self.center_x)?;
        dst.write_f64::<LittleEndian>(self.center_y)?;
        dst.write_f64::<LittleEndian>(self.center_z)?;
        dst.write_f64::<LittleEndian>(self.halfsize)?;
        dst.write_f64::<LittleEndian>(self.spacing)?;
        dst.write_u64::<LittleEndian>(self.root_hier_offset)?;
        dst.write_u64::<LittleEndian>(self.root_hier_size)?;
        dst.write_f64::<LittleEndian>(self.gpstime_minimum)?;
        dst.write_f64::<LittleEndian>(self.gpstime_maximum)?;
        self.reserved
            .into_iter()
            .try_for_each(|i| dst.write_u64::<LittleEndian>(i))?;
        Ok(())
    }
}

impl TryFrom<&Vlr> for CopcInfoVlr {
    type Error = Error;

    fn try_from(value: &Vlr) -> Result<Self> {
        Self::read_from::<&[u8]>(value.data.as_ref())
    }
}

/// VoxelKey corresponds to the naming of EPT data files.
/// <https://entwine.io/en/latest/entwine-point-tile.html#ept-data>
/// The point cloud data itself is arranged in a 3D analogous manner to slippy map tiling schemes.
/// The scheme is Level-X-Y-Z.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct VoxelKey {
    // A value < 0 indicates an invalid VoxelKey
    l: i32,
    x: i32,
    y: i32,
    z: i32,
}
impl VoxelKey {
    /// Computes the Childs of a VoxelKey
    /// There are max 8 Childs to a VoxelKey
    /// **dir**ection needs to be 0..8
    pub fn child(&self, direction: i32) -> Result<Self> {
        // TODO: Maybe direction %= 8; would be better
        if !(0..8).contains(&direction) {
            return Err(Error::FunctionArgumentRequirementsNotMet {
                argument: format!("direction needs to be (0..8). Was {direction}"),
            });
        }
        // bit permutations:
        // 0 -> l+1,2x  ,2y  ,2z
        // 1 -> l+1,2x+1,2y  ,2z
        // 2 -> l+1,2x  ,2y+1,2z
        // 3 -> l+1,2x+1,2y+1,2z
        // ...
        // 7 -> +1,2x+1,2y+1,2z+1
        // TODO: << can overflow to negative
        Ok(Self {
            l: self.l + 1,
            x: (self.x << 1) | (direction & 0x1),
            y: (self.y << 1) | ((direction >> 1) & 0x1),
            z: (self.z << 1) | ((direction >> 2) & 0x1),
        })
    }
    /// Computes the parent VoxelKey
    pub fn parent(&self) -> Self {
        //bitshift back to the upper left parent index
        Self {
            l: 0.max(self.l - 1),
            x: self.x >> 1,
            y: self.y >> 1,
            z: self.z >> 1,
        }
    }
    /// Creates the Root node of a Page
    pub const ROOT: Self = Self {
        l: 0,
        x: 0,
        y: 0,
        z: 0,
    };
    /// Read a VoxelKey from Vlr Payload data
    pub fn read_from<R: Read>(read: &mut R) -> Result<Self> {
        Ok(Self {
            l: read.read_i32::<LittleEndian>()?,
            x: read.read_i32::<LittleEndian>()?,
            y: read.read_i32::<LittleEndian>()?,
            z: read.read_i32::<LittleEndian>()?,
        })
    }

    fn write_to<W: Write>(&self, dst: &mut W) -> Result<()> {
        dst.write_i32::<LittleEndian>(self.l)?;
        dst.write_i32::<LittleEndian>(self.x)?;
        dst.write_i32::<LittleEndian>(self.y)?;
        dst.write_i32::<LittleEndian>(self.z)?;
        Ok(())
    }
}

/// An entry corresponds to a single key/value pair in an EPT hierarchy, but
/// contains additional information to allow direct access and decoding of the
/// corresponding point data.
/// One Entry has 32 bytes
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// EPT key of the data to which this entry corresponds
    pub key: VoxelKey,
    /// Absolute offset to the data chunk if the pointCount > 0.
    /// Absolute offset to a child hierarchy page if the pointCount is -1.
    /// 0 if the pointCount is 0.
    pub offset: u64,
    /// Size of the data chunk in bytes (compressed size) if the pointCount > 0.
    /// Size of the hierarchy page if the pointCount is -1.
    /// 0 if the pointCount is 0.
    pub byte_size: i32,
    /// If > 0, represents the number of points in the data chunk.
    /// If -1, indicates the information for this octree node is found in another hierarchy pag
    /// If 0, no point data exists for this key, though may exist for child entries.
    pub point_count: i32,
}
impl Entry {
    /// Reads hierarchy entry from a `Read`.
    fn read_from<R: Read>(read: &mut R) -> Result<Self> {
        Ok(Self {
            key: VoxelKey::read_from(read)?,
            offset: read.read_u64::<LittleEndian>()?,
            byte_size: read.read_i32::<LittleEndian>()?,
            point_count: read.read_i32::<LittleEndian>()?,
        })
    }
    fn write_to<W: Write>(&self, dst: &mut W) -> Result<()> {
        self.key.write_to(dst)?;
        dst.write_u64::<LittleEndian>(self.offset)?;
        dst.write_i32::<LittleEndian>(self.byte_size)?;
        dst.write_i32::<LittleEndian>(self.point_count)?;
        Ok(())
    }
    fn is_referencing_page(&self) -> bool {
        self.point_count == -1
    }
}

/// The entries of a hierarchy page are consecutive. The number of entries in a page
/// can be determined by taking the size of the page (contained in the parent page as
/// Entry::byteSize or in the COPC info VLR as CopcData::root_hier_size)
/// and dividing by the size of an Entry (32 bytes).
#[derive(Debug)]
struct Page {
    entries: Vec<Entry>, //[page_size / 32];
}
impl Page {
    /// Reads hierarchy page from a `Read`.
    fn read_from(mut data: &[u8]) -> Result<Self> {
        Ok(Self {
            entries: (0..data.len() / 32)
                .map(|_| Entry::read_from(&mut data))
                .collect::<Result<Vec<Entry>>>()?,
        })
    }
    fn write_to<W: Write>(&self, dst: &mut W) -> Result<()> {
        self.entries
            .iter()
            .try_for_each(|entry| entry.write_to(dst))?;
        Ok(())
    }
}

/// the hierarchy VLR MUST exist.
/// Like EPT, COPC stores hierarchy information to allow a reader to locate points
/// that are in a particular octree node. Also like EPT, the hierarchy MAY be
/// arranged in a tree of pages, but SHALL always consist of at least ONE hierarchy
/// page.
/// The VLR data consists of one or more hierarchy pages. Each hierarchy data
/// page is written as follows:
/// The VoxelKey corresponds to the naming of EPT data files.
///
/// The octree hierarchy is arranged in pages. The COPC VLR provides information
/// describing the location and size of root hierarchy page. The root hierarchy page
/// can be used to traverse to child pages. Each entry in a hierarchy page either
/// refers to a child hierarchy page, octree node data chunk, or an empty octree
/// node. The size and file offset of each data chunk is provided in the hierarchy
/// entries, allowing the chunks to be directly read for decoding.
#[derive(Debug)]
pub struct CopcHierarchyVlr {
    root: Page,
    sub_pages: HashMap<VoxelKey, Page>,
}
impl CopcHierarchyVlr {
    /// The record id of the LasZip VLR header.
    pub const RECORD_ID: u16 = 1000;

    /// Writes the Vlr data to the source.
    ///
    /// This **only** writes the *payload data* the
    /// vlr header should be written before-hand.
    pub fn write_to<W: Write>(&self, dst: &mut W) -> Result<()> {
        self.root.write_to(dst)?;
        self.sub_pages
            .iter()
            .try_for_each(|(_, page)| page.write_to(dst))
    }
    /// Reads the CopcHierarchyVlr from the Vlr payload with specifications from copc_info
    pub fn read_from_with(vlr: &Vlr, copc_info: &CopcInfoVlr) -> Result<CopcHierarchyVlr> {
        let root = Page::read_from(vlr.data[0..copc_info.root_hier_size as usize].as_ref())?;
        let sub_pages = root
            .entries
            .iter()
            .filter(|entry| entry.is_referencing_page())
            .map(|entry| {
                let start = (entry.offset - copc_info.root_hier_offset) as usize;
                let end = start + entry.byte_size as usize;
                Page::read_from(vlr.data[start..end].as_ref()).map(|p| (entry.key, p))
            })
            .collect::<Result<HashMap<VoxelKey, Page>>>()?;
        Ok(CopcHierarchyVlr { root, sub_pages })
    }
    /// iterates over all entries merging all referenced pages into root
    pub fn iter_entrys(&self) -> impl Iterator<Item = Entry> {
        self.root.entries.iter().flat_map(|entry| {
            if entry.is_referencing_page() {
                if let Some(page) = self.sub_pages.get(&entry.key) {
                    page.entries.clone()
                } else {
                    // this entry is corrupt or the page is missing
                    vec![Entry {
                        key: entry.key,
                        offset: entry.offset,
                        byte_size: 0,
                        point_count: 0,
                    }]
                }
            } else {
                vec![*entry]
            }
        })
    }
}
impl Vlr {
    /// Returns true if this [Vlr] is the Copc info Vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// #[cfg(feature = "copc")]
    /// {
    /// use las::{copc, Vlr};
    ///
    /// let mut vlr = Vlr::default();
    /// assert!(!copc::is_copcinfo_vlr(&vlr));
    /// vlr.user_id = "copc".to_string();
    /// vlr.record_id = 1;
    /// assert!(copc::is_copcinfo_vlr(&vlr));
    /// }
    /// ```
    pub fn is_copc_info(&self) -> bool {
        self.user_id == USER_ID && self.record_id == CopcInfoVlr::RECORD_ID
    }
}

impl Header {
    /// doc
    pub fn copc_info_vlr(&self) -> Result<CopcInfoVlr> {
        self.vlrs
            .iter()
            .find(|vlr| vlr.is_copc_info())
            .map_or(Err(Error::CopcInfoVlrNotFound), |vlr| vlr.try_into())
    }
}
impl Vlr {
    /// Returns true if this [Vlr] is the Copc Heirarchy Vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// #[cfg(feature = "copc")]
    /// {
    /// use las::{copc, Vlr};
    ///
    /// let mut vlr = Vlr::default();
    /// assert!(!copc::is_copchierarchy_evlr(&vlr));
    /// vlr.user_id = "copc".to_string();
    /// vlr.record_id = 1000;
    /// assert!(copc::is_copchierarchy_evlr(&vlr));
    /// }
    /// ```
    pub fn is_copchierarchy_evlr(&self) -> bool {
        self.user_id == USER_ID && self.record_id == CopcHierarchyVlr::RECORD_ID
    }
}
impl Header {
    /// doc
    pub fn copc_hierarchy_evlr(&self) -> Result<CopcHierarchyVlr> {
        let copc_info = self.copc_info_vlr()?;
        self.evlrs()
            .iter()
            .find(|vlr| vlr.is_copchierarchy_evlr())
            .map_or(Err(Error::CopcHierarchyVlrNotFound), |vlr| {
                CopcHierarchyVlr::read_from_with(vlr, &copc_info)
            })
    }
}
#[cfg(test)]
mod tests {
    use super::{Result, VoxelKey};
    #[test]
    fn test_voxelkey() {
        let vk = VoxelKey::ROOT;
        let childs = (0..8)
            .map(|dir| vk.child(dir))
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert!(childs
            .iter()
            .map(|v| v.parent())
            .all(|v| v.eq(&VoxelKey::ROOT)));
        assert!(childs
            .iter()
            .map(|c| (
                c,
                (0..8).map(|dir| c.child(dir).unwrap()).collect::<Vec<_>>()
            ))
            .all(|(p, childs)| childs.iter().all(|c| c.parent().eq(p))));
    }

    #[test]
    fn test_vlr_copc_autzen() {
        let reader =
            crate::Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let copcinfo = reader.header().copc_info_vlr().unwrap();
        let copchier = reader.header().copc_hierarchy_evlr().unwrap();
        assert!(copcinfo.root_hier_offset == 4336);
        assert!(copcinfo.root_hier_size == 32);
        assert!(copchier.root.entries[0].key == VoxelKey::ROOT);
    }

    #[test]
    fn test_copc_entry_key_autzen() {
        let reader =
            crate::Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let root_entry = reader
            .header()
            .copc_hierarchy_evlr()
            .unwrap()
            .iter_entrys()
            .next()
            .unwrap();
        assert_eq!(root_entry.key, VoxelKey::ROOT);
        assert_eq!(root_entry.point_count, 107);
    }
}
