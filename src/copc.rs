//! [COPC](https://copc.io/) header data

use crate::{raw, Point};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use laz::record::{LayeredPointRecordDecompressor, RecordDecompressor};
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek, SeekFrom, Write},
};

/// The user id of the LasZip VLR header.
pub const USER_ID: &str = "copc";

/// The description of the LasZip VLR header.
pub const DESCRIPTION: &str = "https://copc.io";

use crate::{Error, Header, Result, Vlr};

/// The COPC Info Vlr.
///
/// Requirements:
///
/// - The info VLR MUST exist.
/// - The info VLR MUST be the first VLR in the file (must begin at offset 375
///   from the beginning of the file).
/// - The info VLR is 160 bytes described by the following structure. reserved
///   elements MUST be set to 0.
#[derive(Debug)]
pub struct CopcInfoVlr {
    /// Actual (unscaled) X coordinate of center of octree
    pub center_x: f64,
    /// Actual (unscaled) Y coordinate of center of octree
    pub center_y: f64,
    /// Actual (unscaled) Z coordinate of center of octree
    pub center_z: f64,
    /// Perpendicular distance from the center to any side of the root node.
    pub halfsize: f64,
    /// Space between points at the root node.
    /// This value is halved at each octree level
    pub spacing: f64,
    // File offset to the first hierarchy page
    root_hier_offset: u64,
    // Size of the first hierarchy page in bytes
    root_hier_size: u64,
    /// Minimum of GPSTime
    pub gpstime_minimum: f64,
    /// Maximum of GPSTime
    pub gpstime_maximum: f64,
    // Must be 0
    reserved: [u64; 11],
}

impl CopcInfoVlr {
    /// The record id of the LasZip VLR header.
    pub const RECORD_ID: u16 = 1;

    /// Reads the Vlr data from the source.
    ///
    /// This only reads the payload data, the vlr header should already be read.
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
    /// This only writes the payload data the vlr header should be written
    /// before-hand.
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
///
/// See <https://entwine.io/en/latest/entwine-point-tile.html#ept-data> for more.
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
    /// Computes a child of a VoxelKey
    ///
    /// There are max 8 Childs to a VoxelKey, direction must be in 0..8.
    pub fn child(&self, direction: i32) -> Result<Self> {
        // TODO: Maybe direction %= 8; would be better
        if !(0..8).contains(&direction) {
            return Err(Error::InvalidDirection(direction));
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

    /// Computes the parent VoxelKey.
    pub fn parent(&self) -> Self {
        Self {
            l: 0.max(self.l - 1),
            x: self.x >> 1,
            y: self.y >> 1,
            z: self.z >> 1,
        }
    }

    /// The root voxel key.
    pub const ROOT: Self = Self {
        l: 0,
        x: 0,
        y: 0,
        z: 0,
    };

    /// Read a VoxelKey from Vlr Payload data.
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
///
/// One Entry has 32 bytes
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// EPT key of the data to which this entry corresponds
    pub key: VoxelKey,

    /// Absolute offset to the data chunk if the pointCount > 0.
    ///
    /// Absolute offset to a child hierarchy page if the pointCount is -1.
    /// 0 if the pointCount is 0.
    pub offset: u64,

    /// Size of the data chunk in bytes (compressed size) if the pointCount > 0.
    ///
    /// Size of the hierarchy page if the pointCount is -1.
    /// 0 if the pointCount is 0.
    pub byte_size: i32,

    /// If > 0, represents the number of points in the data chunk.
    ///
    /// If -1, indicates the information for this octree node is found in another hierarchy pag
    /// If 0, no point data exists for this key, though may exist for child entries.
    pub point_count: i32,
}

impl Entry {
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

/// The entries of a hierarchy page are consecutive.
///
/// The number of entries in a page can be determined by taking the size of the
/// page (contained in the parent page as [Entry::byte_size] or in the COPC info
/// VLR as [CopcData::root_hier_size]) and dividing by the size of an Entry (32
/// bytes).
#[derive(Debug)]
struct Page {
    entries: Vec<Entry>,
}

impl Page {
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

/// The hierarchy VLR MUST exist.
///
/// Like EPT, COPC stores hierarchy information to allow a reader to locate
/// points that are in a particular octree node. Also like EPT, the hierarchy
/// MAY be arranged in a tree of pages, but SHALL always consist of at least ONE
/// hierarchy page.  The VLR data consists of one or more hierarchy pages. Each
/// hierarchy data page is written as follows:
///
/// VoxelKey corresponds to the naming of EPT data files.  octree hierarchy is
/// arranged in pages. The COPC VLR provides information describing the location
/// and size of root hierarchy page. The root hierarchy page can be used to
/// traverse to child pages. Each entry in a hierarchy page either refers to a
/// child hierarchy page, octree node data chunk, or an empty octree node. The
/// size and file offset of each data chunk is provided in the hierarchy
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

    /// Reads the CopcHierarchyVlr from the Vlr payload with specifications from copc_info.
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
    pub fn iter_entries(&self) -> EntryIterator<'_> {
        EntryIterator::new(self.root.entries.iter().peekable(), &self.sub_pages)
    }
}

/// An iterator over COPC entries that handles references to sub-pages.
///
/// This iterator provides a flattened view of all entries in a COPC hierarchy,
/// transparently resolving references to sub-pages. It returns borrowed references
/// to entries rather than cloning them, improving performance when iterating over
/// large hierarchies.
///
/// When encountering an entry that references a sub-page, the iterator will:
///
/// 1. Look up the referenced page in the provided sub-pages HashMap
/// 2. Iterate through all entries in that page
/// 3. Continue with the next root entry
///
/// If a referenced page is missing, the iterator will return an error containing
/// the problematic entry.
#[derive(Debug)]
pub struct EntryIterator<'a> {
    /// Peekable iterator over root entries, allows looking ahead without consuming
    root_iter: std::iter::Peekable<std::slice::Iter<'a, Entry>>,

    /// Optional iterator over entries in the currently referenced page
    ref_iter: Option<std::slice::Iter<'a, Entry>>,

    /// Reference to the mapping of VoxelKeys to Pages containing sub-entries
    sub_pages: &'a HashMap<VoxelKey, Page>,
}

impl<'a> EntryIterator<'a> {
    fn new(
        root_iter: std::iter::Peekable<std::slice::Iter<'a, Entry>>,
        sub_pages: &'a HashMap<VoxelKey, Page>,
    ) -> Self {
        Self {
            root_iter,
            ref_iter: None,
            sub_pages,
        }
    }
}
impl<'a> Iterator for EntryIterator<'a> {
    type Item = Result<&'a Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match (
                &mut self.ref_iter,
                self.root_iter
                    .peek()
                    .map(|entry| entry.is_referencing_page()),
            ) {
                // there currently is no page referenced and the next root entry would reference a page
                (None, Some(true)) => {
                    let next_entry = self.root_iter.next();
                    self.ref_iter = next_entry
                        .and_then(|entry| self.sub_pages.get(&entry.key))
                        .map(|page| page.entries.iter());
                    if self.ref_iter.is_none() {
                        // Entry is referencing a  missing page
                        return next_entry
                            .map(|entry| Err(Error::ReferencedPageMissingFromEvlr(*entry)));
                    }
                }
                //there is a page referenced
                (Some(ref_iter), _) => {
                    if let Some(entry) = ref_iter.next() {
                        return Some(Ok(entry));
                    } else {
                        //iterator is empty
                        self.ref_iter = None;
                    }
                }
                // there is no page referenced and the next entry would not reference a page
                (None, Some(false)) => return self.root_iter.next().map(Ok),
                // the root iterator is empty
                (None, None) => return None,
            }
        }
    }
}
impl Vlr {
    /// Returns true if this [Vlr] is the Copc info Vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    ///
    /// let mut vlr = Vlr::default();
    /// assert!(!vlr.is_copc_info());
    /// vlr.user_id = "copc".to_string();
    /// vlr.record_id = 1;
    /// assert!(&vlr.is_copc_info());
    /// ```
    pub fn is_copc_info(&self) -> bool {
        self.user_id == USER_ID && self.record_id == CopcInfoVlr::RECORD_ID
    }
}

impl Header {
    /// Retrieves the COPC Info VLR (Variable Length Record) if available.
    ///
    /// This function searches through the available VLRs to find the COPC Info VLR.
    ///
    /// # Returns
    ///
    /// * `Some(CopcInfolr)` - If the COPC Info VLR exists and can be successfully parsed
    /// * `None` - If the COPC Info VLR doesn't exist or if there was an error parsing it.
    pub fn copc_info_vlr(&self) -> Option<CopcInfoVlr> {
        self.vlrs
            .iter()
            .find(|vlr| vlr.is_copc_info())
            .and_then(|vlr| vlr.try_into().ok())
    }
}
impl Vlr {
    /// Returns true if this [Vlr] is the Copc Hierarchy Vlr.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Vlr;
    ///
    /// let mut vlr = Vlr::default();
    /// assert!(!vlr.is_copc_hierarchy());
    /// vlr.user_id = "copc".to_string();
    /// vlr.record_id = 1000;
    /// assert!(vlr.is_copc_hierarchy());
    /// ```
    pub fn is_copc_hierarchy(&self) -> bool {
        self.user_id == USER_ID && self.record_id == CopcHierarchyVlr::RECORD_ID
    }
}

impl Header {
    /// Retrieves the COPC hierarchy EVLR (Extended Variable Length Record) if available.
    ///
    /// This function searches through the available EVLRs to find the COPC hierarchy EVLR,
    /// and then attempts  to parse it using the COPC info VLR.
    ///
    /// # Returns
    ///
    /// * `Some(CopcHierarchyVlr)` - If the COPC hierarchy EVLR exists and can be successfully parsed
    /// * `None` - If the COPC info VLR doesn't exist, the COPC hierarchy EVLR doesn't exist,
    ///   or if there was an error parsing the COPC hierarchy EVLRto parse it using the CopcInfoVlr.
    pub fn copc_hierarchy_evlr(&self) -> Option<CopcHierarchyVlr> {
        let copc_info = self.copc_info_vlr()?;
        self.evlrs()
            .iter()
            .find(|vlr| vlr.is_copc_hierarchy())
            .and_then(|vlr| CopcHierarchyVlr::read_from_with(vlr, &copc_info).ok())
    }
}

/// Entry Reader can read whole entries of copc laz files
/// A reader for COPC (Cloud Optimized Point Cloud) entries that handles decompression and point reading.
///
/// This struct provides functionality to read points from COPC entries in LAZ files,
/// handling the necessary decompression and format transformations.
#[allow(missing_debug_implementations)]
pub struct CopcEntryReader<'a, R: Read + Seek> {
    decompressor: LayeredPointRecordDecompressor<'a, R>,
    buffer: Cursor<Vec<u8>>,
    header: Header,
}

impl<R: Read + Seek> CopcEntryReader<'_, R> {
    /// Creates a new COPC Entry reader.
    ///
    /// Initializes a new reader by parsing the LAS header and setting up the decompressor
    /// with the appropriate field configurations from the LAZ VLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::CopcEntryReader;
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let reader = CopcEntryReader::new(file).unwrap();
    /// ```
    pub fn new(mut read: R) -> Result<Self> {
        let header = Header::new(read.by_ref())?;
        let mut decompressor = LayeredPointRecordDecompressor::new(read);
        decompressor.set_fields_from(header.laz_vlr()?.items())?;
        let buffer = Cursor::new(Vec::new());
        Ok(Self {
            decompressor,
            buffer,
            header,
        })
    }

    /// Retrieves all entries from the COPC hierarchy.
    ///
    /// This method extracts all COPC hierarchy entries from the Extended Variable Length Record (EVLR)
    /// in the file header, providing access to the octree structure of the point cloud.
    ///
    /// # Notes
    ///
    /// The method filters out any entries that could not be parsed correctly, returning only
    /// successfully parsed entries.
    pub fn hierarchy_entries(&self) -> Option<Vec<Entry>> {
        self.header()
            .copc_hierarchy_evlr()
            .map(|vlr| vlr.iter_entries().filter_map(|e| e.ok().copied()).collect())
    }

    /// Reads all points specified by a COPC entry.
    ///
    /// Seeks to the specified offset in the file, decompresses the point data,
    /// and converts the raw points to the point format defined by the header.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::CopcEntryReader;
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let mut entry_reader = CopcEntryReader::new(file).unwrap();
    /// // Get entry from hierarchy
    /// let root_entry = entry_reader.hierarchy_entries().unwrap()[0];
    /// // Read all points
    /// let mut points = Vec::new();
    /// let point_count = entry_reader.read_entry_points(&root_entry, &mut points).unwrap();
    /// println!("Read {} points", point_count);
    /// ```
    pub fn read_entry_points(&mut self, entry: &Entry, points: &mut Vec<Point>) -> Result<u64> {
        let _off = self
            .decompressor
            .get_mut()
            .seek(SeekFrom::Start(entry.offset))?;
        points.reserve_exact(entry.point_count as usize);

        let resize = usize::try_from(
            entry.point_count as u64 * u64::from(self.header.point_format().len()),
        )?;
        self.buffer.get_mut().resize(resize, 0u8);
        self.decompressor.decompress_many(self.buffer.get_mut())?;
        self.buffer.set_position(0);
        points.reserve(entry.point_count as usize);

        for _ in 0..entry.point_count as usize {
            let point = raw::Point::read_from(&mut self.buffer, self.header.point_format())
                .map(|raw_point| Point::new(raw_point, self.header.transforms()))?;
            points.push(point);
        }
        Ok(entry.point_count as u64)
    }

    /// Returns a reference to the LAS header.
    ///
    /// Provides access to the header information of the LAS/LAZ file,
    /// which contains metadata about the point cloud.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::CopcEntryReader;
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let reader = CopcEntryReader::new(file).unwrap();
    /// let header = reader.header();
    /// println!("Point count: {}", header.number_of_points());
    /// println!("Point format: {:?}", header.point_format());
    /// ```
    pub fn header(&self) -> &Header {
        &self.header
    }
}

#[cfg(test)]
mod tests {
    use super::{Result, VoxelKey};
    use crate::{copc::CopcEntryReader, Reader};
    use std::{fs::File, io::BufReader};
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
        let reader = Reader::from_path("tests/data/autzen.copc.laz").expect("Cannot open reader");
        let copcinfo = reader.header().copc_info_vlr().unwrap();
        let copchier = reader.header().copc_hierarchy_evlr().unwrap();
        assert!(copcinfo.root_hier_offset == 4336);
        assert!(copcinfo.root_hier_size == 32);
        assert!(copchier.root.entries[0].key == VoxelKey::ROOT);
    }

    #[test]
    fn test_copc_entry_key_autzen() {
        let file =
            BufReader::new(File::open("tests/data/autzen.copc.laz").expect("Cannot open reader"));
        let entry_reader = CopcEntryReader::new(file).unwrap();
        let root_entry = entry_reader.hierarchy_entries().unwrap()[0];
        assert_eq!(root_entry.key, VoxelKey::ROOT);
        assert_eq!(root_entry.point_count, 107);
    }

    #[test]
    fn test_copc_read_autzen() {
        let copc_points = {
            let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
            let mut entry_reader = CopcEntryReader::new(file).unwrap();
            let root_entry = entry_reader.hierarchy_entries().unwrap()[0];
            let mut points = Vec::new();
            let _p_num = entry_reader
                .read_entry_points(&root_entry, &mut points)
                .unwrap();
            points
        };
        let mut laz_points = Vec::new();
        let _pnum = Reader::from_path("tests/data/autzen.copc.laz")
            .unwrap()
            .read_all_points_into(&mut laz_points)
            .unwrap();
        assert!(laz_points
            .iter()
            .zip(copc_points)
            .all(|(laz_point, copc_point)| laz_point.eq(&copc_point)));
    }
}
