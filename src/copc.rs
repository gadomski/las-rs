//! [COPC](https://copc.io/) header data

use crate::{Bounds, Error, Header, PointData, PointDataBuilder, Result, Vector, Vlr};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use laz::record::{LayeredPointRecordDecompressor, RecordDecompressor};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufReader, Cursor, Read, Seek, SeekFrom, Write},
    ops::Range,
    path::Path,
};

/// The COPC Info Vlr.
///
/// Requirements:
///
/// - The info VLR MUST exist.
/// - The info VLR MUST be the first VLR in the file (must begin at offset 375 from the beginning of the file).
/// - The info VLR is 160 bytes described by the following structure. reserved elements MUST be set to 0.
#[derive(Clone, Debug)]
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
    /// The record id of the CopcInfo VLR header.
    pub const RECORD_ID: u16 = 1;
    /// The user id of the CopcInfo VLR header.
    pub const USER_ID: &str = "copc";

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
    /// This only writes the payload data
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
        if value.record_id == Self::RECORD_ID && value.user_id == Self::USER_ID {
            Self::read_from::<&[u8]>(value.data.as_ref())
        } else {
            Err(Error::CopcInfoVlrNotFound)
        }
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
    /// The level of detail
    pub l: i32,
    #[allow(missing_docs)]
    pub x: i32,
    #[allow(missing_docs)]
    pub y: i32,
    #[allow(missing_docs)]
    pub z: i32,
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

    /// Compute all 8 children of the VoxelKey
    pub fn children(&self) -> impl Iterator<Item = Self> {
        (0..8).map(|i| self.child(i).unwrap())
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

    /// Calculates bounds of the VoxelKey.
    /// Serves as a guidance implementation.
    pub fn bounds(&self, copc_info: &CopcInfoVlr) -> Bounds {
        let root_min_x = copc_info.center_x - copc_info.halfsize;
        let root_min_y = copc_info.center_y - copc_info.halfsize;
        let root_min_z = copc_info.center_z - copc_info.halfsize;

        let root_max_x = copc_info.center_x + copc_info.halfsize;
        let root_max_y = copc_info.center_y + copc_info.halfsize;
        let root_max_z = copc_info.center_z + copc_info.halfsize;

        let root_size_x = root_max_x - root_min_x;
        let root_size_y = root_max_y - root_min_y;
        let root_size_z = root_max_z - root_min_z;

        let voxel_size_x = root_size_x / (1 << self.l) as f64;
        let voxel_size_y = root_size_y / (1 << self.l) as f64;
        let voxel_size_z = root_size_z / (1 << self.l) as f64;

        let voxel_min = Vector {
            x: root_min_x + voxel_size_x * self.x as f64,
            y: root_min_y + voxel_size_y * self.y as f64,
            z: root_min_z + voxel_size_z * self.z as f64,
        };

        let voxel_max = Vector {
            x: voxel_min.x + voxel_size_x,
            y: voxel_min.y + voxel_size_y,
            z: voxel_min.z + voxel_size_z,
        };
        Bounds {
            min: voxel_min,
            max: voxel_max,
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
    fn read_from<R: Read>(read: &mut R) -> Result<Self> {
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
#[derive(Debug, Clone)]
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
/// Like EPT, COPC stores hierarchy information to allow a reader to locate points that are in a particular octree node.
/// Also like EPT, the hierarchy MAY be arranged in a tree of pages, but SHALL always consist of at least ONE hierarchy page.
/// The VLR data consists of one or more hierarchy pages. Each hierarchy data page is written as follows:
///
/// VoxelKey corresponds to the naming of EPT data files.
/// The octree hierarchy is arranged in pages.
/// The COPC VLR provides information describing the location and size of root hierarchy page.
/// The root hierarchy page can be used to traverse to child pages.
/// Each entry in a hierarchy page either refers to a child hierarchy page, octree node data chunk, or an empty octree node.
/// The size and file offset of each data chunk is provided in the hierarchy entries, allowing the chunks to be directly read for decoding.
#[derive(Clone, Debug)]
pub struct CopcHierarchyVlr {
    root: Page,
    sub_pages: HashMap<VoxelKey, Page>,
}

impl CopcHierarchyVlr {
    /// The record id of the CopcHierarchy VLR header.
    pub const RECORD_ID: u16 = 1000;
    /// The user id of the CopcHierarchy VLR header.
    pub const USER_ID: &str = "copc";

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
        if vlr.record_id != Self::RECORD_ID || vlr.user_id != Self::USER_ID {
            return Err(Error::CopcHierarchyEvlrNotFound);
        }

        let read_page = |offset: u64, byte_size: u64| {
            let start: usize = offset
                .checked_sub(copc_info.root_hier_offset)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid COPC page: Root page should be the first Page in the Copc Hierarchy EVLR",
                    )
                })?.try_into()?;
            let end = start
                .checked_add(usize::try_from(byte_size)?)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid COPC page: Page end address overflows usize",
                    )
                })?;
            let data = vlr.data.get(start..end).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid COPC page: Page is longer than the Copc Hierarchy EVLR",
                )
            })?;
            Page::read_from(data)
        };
        let root = read_page(copc_info.root_hier_offset, copc_info.root_hier_size)?;
        let mut sub_pages = HashMap::new();
        let mut pending = root
            .entries
            .iter()
            .filter(|entry| entry.is_referencing_page())
            .copied()
            .collect::<Vec<_>>();

        while let Some(entry) = pending.pop() {
            if sub_pages.contains_key(&entry.key) {
                continue;
            }
            let page = read_page(entry.offset, u64::try_from(entry.byte_size)?)?;
            pending.extend(
                page.entries
                    .iter()
                    .filter(|entry| entry.is_referencing_page()),
            );
            let _ = sub_pages.insert(entry.key, page);
        }
        Ok(CopcHierarchyVlr { root, sub_pages })
    }

    /// iterates over all entries merging all referenced pages into root
    pub fn iter_entries(&self) -> EntryIterator<'_> {
        EntryIterator::new(self.root.entries.iter(), &self.sub_pages)
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
    /// Stack of iterators for the hierarchy pages currently being traversed.
    iterators: Vec<std::slice::Iter<'a, Entry>>,

    /// Reference to the mapping of VoxelKeys to Pages containing sub-entries
    sub_pages: &'a HashMap<VoxelKey, Page>,
}

impl<'a> EntryIterator<'a> {
    fn new(root_iter: std::slice::Iter<'a, Entry>, sub_pages: &'a HashMap<VoxelKey, Page>) -> Self {
        Self {
            iterators: vec![root_iter],
            sub_pages,
        }
    }
}

impl<'a> Iterator for EntryIterator<'a> {
    type Item = Result<&'a Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.iterators.last_mut()?.next() {
                Some(entry) => entry,
                None => {
                    let _ = self.iterators.pop();
                    continue;
                }
            };
            if entry.is_referencing_page() {
                match self.sub_pages.get(&entry.key) {
                    Some(page) => self.iterators.push(page.entries.iter()),
                    None => return Some(Err(Error::ReferencedPageMissingFromEvlr(*entry))),
                }
            } else {
                return Some(Ok(entry));
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
        self.user_id == CopcInfoVlr::USER_ID && self.record_id == CopcInfoVlr::RECORD_ID
    }
}

impl Header {
    /// Retrieves the COPC Info VLR (Variable Length Record) if available.
    ///
    /// This function searches through the available VLRs to find the COPC Info VLR.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(CopcInfoVlr))` - If the COPC Info VLR exists and can be successfully parsed
    /// * `Ok(None)` - If the COPC Info VLR doesn't exist.
    /// * `Err(crate::Error)` - If the COPC Info VLR exists, but parsing failed
    pub fn copc_info_vlr(&self) -> Result<Option<CopcInfoVlr>> {
        self.vlrs
            .iter()
            .find(|vlr| vlr.is_copc_info())
            .map(|vlr| vlr.try_into())
            .transpose()
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
        self.user_id == CopcHierarchyVlr::USER_ID && self.record_id == CopcHierarchyVlr::RECORD_ID
    }
}

impl Header {
    /// Retrieves the COPC hierarchy EVLR (Extended Variable Length Record) if available.
    ///
    /// This function searches through the available EVLRs to find the COPC hierarchy EVLR,
    /// and then attempts to parse it using the COPC info VLR.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(CopcHierarchyVlr))` - If the COPC hierarchy EVLR exists and is successfully parsed
    /// * `Ok(None)` - If the COPC info VLR doesn't exist or the COPC hierarchy EVLR doesn't exist,
    /// * `Err(Error)` - If the COPC hierarchy EVLR exists, but parsing fails
    pub fn copc_hierarchy_evlr(&self) -> Result<Option<CopcHierarchyVlr>> {
        let Some(copc_info) = self.copc_info_vlr()? else {
            return Ok(None);
        };
        self.evlrs()
            .iter()
            .find(|vlr| vlr.is_copc_hierarchy())
            .map(|vlr| CopcHierarchyVlr::read_from_with(vlr, &copc_info))
            .transpose()
    }
}

/// Selects the octree levels returned by a COPC query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LodSelection {
    /// All levels.
    All,
    /// Levels needed to provide at least the requested point spacing.
    Resolution(f64),
    /// Only the requested level.
    Level(i32),
    /// Levels in the half-open range `min..max`.
    LevelMinMax(i32, i32),
}

/// Selects the bounds returned by a COPC query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoundsSelection {
    /// No bounds filter.
    All,
    /// Points within the bounds.
    Within(Bounds),
}

/// Reads and queries points from a COPC file.
#[allow(missing_debug_implementations)]
pub struct CopcReader<'a, R: Read + Seek> {
    decompressor: LayeredPointRecordDecompressor<'a, R>,
    buffer: Cursor<Vec<u8>>,
    header: Header,
    copc_info: CopcInfoVlr,
    hierarchy: HashMap<VoxelKey, Entry>,
}

impl CopcReader<'static, BufReader<File>> {
    /// Creates a COPC reader from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        File::open(path)
            .map_err(Error::from)
            .and_then(|file| Self::new(BufReader::new(file)))
    }
}

impl<R: Read + Seek> CopcReader<'_, R> {
    /// Creates a new COPC reader.
    ///
    /// Initializes a new reader by parsing the LAS header and setting up the decompressor
    /// with the appropriate field configurations from the LAZ VLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::CopcReader;
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let reader = CopcReader::new(file).unwrap();
    /// ```
    pub fn new(mut read: R) -> Result<Self> {
        let header = Header::new(read.by_ref())?;
        let copc_info = header.copc_info_vlr()?.ok_or(Error::CopcInfoVlrNotFound)?;
        let hierarchy = header
            .copc_hierarchy_evlr()?
            .ok_or(Error::CopcHierarchyEvlrNotFound)?;
        let hierarchy = hierarchy.iter_entries().collect::<Result<Vec<_>>>()?;
        let hierarchy = hierarchy
            .into_iter()
            .map(|e| (e.key, *e))
            .collect::<HashMap<_, _>>();

        let mut decompressor = LayeredPointRecordDecompressor::new(read);
        decompressor.set_fields_from(header.laz_vlr()?.items())?;
        let buffer = Cursor::new(Vec::new());
        Ok(Self {
            decompressor,
            buffer,
            header,
            copc_info,
            hierarchy,
        })
    }

    /// Retrieves all entries from the COPC hierarchy.
    ///
    /// The entries are in no particular order
    pub fn hierarchy_entries(&self) -> impl Iterator<Item = Entry> {
        self.hierarchy.values().copied()
    }

    /// Get a specific entry from the hierarchy by key
    pub fn hierarchy_entry(&self, key: &VoxelKey) -> Option<Entry> {
        self.hierarchy.get(key).copied()
    }

    /// Reads all points specified by a COPC entry.
    ///
    /// The result uses the same [`PointData`] representation as [`crate::Reader`].
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{CopcReader, copc::VoxelKey};
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let mut entry_reader = CopcReader::new(file).unwrap();
    /// // Get entry from hierarchy
    /// let root_entry = entry_reader.hierarchy_entry(&VoxelKey::ROOT).unwrap();
    /// // Read all points
    /// let point_count = entry_reader.read_entry(&root_entry).unwrap().points().len();
    /// println!("Read {} points", point_count);
    /// ```
    pub fn read_entry(&mut self, entry: &Entry) -> Result<PointData> {
        let bytes = self.read_entry_bytes(entry)?.to_vec();

        PointDataBuilder::new()
            .for_header(&self.header)
            .build_from_bytes(bytes)
    }

    /// Reads all points matching the level-of-detail and bounds selections.
    ///
    /// The result uses the same [`PointData`] representation as [`crate::Reader`].
    ///
    /// # Examples
    ///
    /// ```
    /// use las::{BoundsSelection, CopcReader, LodSelection};
    /// let mut reader = CopcReader::from_path("tests/data/autzen.copc.laz").unwrap();
    /// let points = reader
    ///     .query(LodSelection::Level(0), BoundsSelection::All)
    ///     .unwrap();
    /// assert_eq!(points.len(), 107);
    /// ```
    pub fn query(&mut self, levels: LodSelection, bounds: BoundsSelection) -> Result<PointData> {
        let level_range = match levels {
            LodSelection::All => 0..i32::MAX,
            LodSelection::Resolution(resolution) => {
                if !resolution.is_normal() || !resolution.is_sign_positive() {
                    return Err(Error::InvalidResolution(resolution));
                }
                let max = 1.max((self.copc_info.spacing / resolution).log2().ceil() as i32 + 1);
                0..max
            }
            LodSelection::Level(level) => level..level + 1,
            LodSelection::LevelMinMax(min, max) => min..max,
        };
        let query_bounds = match bounds {
            BoundsSelection::All => None,
            BoundsSelection::Within(bounds) => Some(bounds),
        };
        let mut entries =
            select_entries(&self.hierarchy, &self.copc_info, level_range, query_bounds)?;
        entries.sort_by_key(|entry| entry.offset);

        let format = *self.header.point_format();
        let transforms = *self.header.transforms();
        let record_len = usize::from(format.len());

        let total_entry_points = entries
            .iter()
            .fold(0, |acc, e| acc + e.point_count.try_into().unwrap_or(0));

        let mut bytes = Vec::with_capacity(total_entry_points * record_len);
        if let Some(query_bounds) = query_bounds {
            for entry in entries {
                for point in self.read_entry_bytes(&entry)?.chunks_exact(record_len) {
                    if point_in_bounds(point, &query_bounds, &transforms) {
                        bytes.extend_from_slice(point);
                    }
                }
            }
            bytes.shrink_to_fit();
        } else {
            for entry in entries {
                bytes.extend_from_slice(self.read_entry_bytes(&entry)?);
            }
        }
        PointDataBuilder::new()
            .for_header(&self.header)
            .build_from_bytes(bytes)
    }

    fn read_entry_bytes(&mut self, entry: &Entry) -> Result<&[u8]> {
        let point_count = usize::try_from(entry.point_count)?;
        let record_len = usize::from(self.header.point_format().len());
        let byte_count = usize::try_from(u64::try_from(point_count)? * u64::try_from(record_len)?)?;
        let _ = self
            .decompressor
            .get_mut()
            .seek(SeekFrom::Start(entry.offset))?;
        self.decompressor.reset();
        self.decompressor
            .set_fields_from(self.header.laz_vlr()?.items())?;
        self.buffer.get_mut().resize(byte_count, 0u8);
        self.decompressor.decompress_many(self.buffer.get_mut())?;
        Ok(self.buffer.get_ref())
    }

    /// Returns a reference to the LAS header.
    ///
    /// Provides access to the header information of the LAS/LAZ file,
    /// which contains metadata about the point cloud.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::CopcReader;
    /// use std::{fs::File, io::BufReader};
    /// let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
    /// let reader = CopcReader::new(file).unwrap();
    /// let header = reader.header();
    /// println!("Point count: {}", header.number_of_points());
    /// println!("Point format: {:?}", header.point_format());
    /// ```
    pub fn header(&self) -> &Header {
        &self.header
    }
}

/// Backwards-compatible name for [`CopcReader`].
#[deprecated(note = "Use CopcReader instead")]
pub type CopcEntryReader<'a, R> = CopcReader<'a, R>;

fn select_entries(
    entries: &HashMap<VoxelKey, Entry>,
    copc_info: &CopcInfoVlr,
    levels: Range<i32>,
    bounds: Option<Bounds>,
) -> Result<Vec<Entry>> {
    let mut selected = Vec::new();
    let mut pending = vec![VoxelKey::ROOT];

    while let Some(key) = pending.pop() {
        if key.l >= levels.end {
            continue;
        }
        let Some(entry) = entries.get(&key) else {
            continue;
        };
        if bounds.is_some_and(|bounds| !key.bounds(copc_info).intersect(&bounds)) {
            continue;
        }
        if key.l + 1 < levels.end {
            pending.extend(key.children());
        }
        if entry.point_count > 0 && levels.contains(&key.l) {
            selected.push(*entry);
        }
    }
    Ok(selected)
}

fn point_in_bounds(point: &[u8], bounds: &Bounds, transforms: &Vector<crate::Transform>) -> bool {
    let x = transforms.x.direct(i32::from_le_bytes(
        point[0..4]
            .try_into()
            .expect("the four bytes of x-component"),
    ));
    let y = transforms.y.direct(i32::from_le_bytes(
        point[4..8]
            .try_into()
            .expect("the four bytes of y-component"),
    ));
    let z = transforms.z.direct(i32::from_le_bytes(
        point[8..12]
            .try_into()
            .expect("the four bytes of z-component"),
    ));
    bounds.min.x <= x
        && bounds.max.x >= x
        && bounds.min.y <= y
        && bounds.max.y >= y
        && bounds.min.z <= z
        && bounds.max.z >= z
}

#[cfg(test)]
mod tests {

    use super::{
        select_entries, BoundsSelection, CopcHierarchyVlr, CopcInfoVlr, Entry, LodSelection,
        Result, VoxelKey,
    };
    use crate::{Bounds, CopcReader, Reader, Vector, Vlr};
    use std::{collections::HashMap, fs::File, io::BufReader};
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
        let copcinfo = reader.header().copc_info_vlr().unwrap().unwrap();
        let copchier = reader.header().copc_hierarchy_evlr().unwrap().unwrap();
        assert!(copcinfo.root_hier_offset == 4336);
        assert!(copcinfo.root_hier_size == 32);
        assert!(copchier.root.entries[0].key == VoxelKey::ROOT);
    }

    #[test]
    fn test_nested_hierarchy_pages() {
        let child = VoxelKey::ROOT.child(0).unwrap();
        let entries = [
            Entry {
                key: VoxelKey::ROOT,
                offset: 132,
                byte_size: 64,
                point_count: -1,
            },
            Entry {
                key: VoxelKey::ROOT,
                offset: 1_000,
                byte_size: 20,
                point_count: 1,
            },
            Entry {
                key: child,
                offset: 196,
                byte_size: 32,
                point_count: -1,
            },
            Entry {
                key: child,
                offset: 2_000,
                byte_size: 40,
                point_count: 2,
            },
        ];
        let mut data = Vec::new();
        entries
            .iter()
            .try_for_each(|entry| entry.write_to(&mut data))
            .unwrap();
        let vlr = Vlr {
            data,
            user_id: CopcHierarchyVlr::USER_ID.to_owned(),
            record_id: CopcHierarchyVlr::RECORD_ID,
            ..Default::default()
        };
        let info = CopcInfoVlr {
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            halfsize: 1.0,
            spacing: 1.0,
            root_hier_offset: 100,
            root_hier_size: 32,
            gpstime_minimum: 0.0,
            gpstime_maximum: 0.0,
            reserved: [0; 11],
        };
        let hierarchy = CopcHierarchyVlr::read_from_with(&vlr, &info).unwrap();
        let entries = hierarchy
            .iter_entries()
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].point_count, 1);
        assert_eq!(entries[1].point_count, 2);
    }

    #[test]
    fn test_entry_selection_stops_at_missing_parent() {
        let orphan = VoxelKey::ROOT.child(0).unwrap().child(0).unwrap();
        let entries = vec![
            Entry {
                key: VoxelKey::ROOT,
                offset: 1_000,
                byte_size: 20,
                point_count: 1,
            },
            Entry {
                key: orphan,
                offset: 2_000,
                byte_size: 20,
                point_count: 1,
            },
        ];
        let index = entries
            .into_iter()
            .map(|entry| (entry.key, entry))
            .collect::<HashMap<_, _>>();
        let info = CopcInfoVlr {
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            halfsize: 1.0,
            spacing: 1.0,
            root_hier_offset: 0,
            root_hier_size: 0,
            gpstime_minimum: 0.0,
            gpstime_maximum: 0.0,
            reserved: [0; 11],
        };

        let selected = select_entries(&index, &info, 0..i32::MAX, None).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].key, VoxelKey::ROOT);
    }

    #[test]
    fn test_copc_entry_key_autzen() {
        let file =
            BufReader::new(File::open("tests/data/autzen.copc.laz").expect("Cannot open reader"));
        let entry_reader = CopcReader::new(file).unwrap();
        let root_entry = entry_reader.hierarchy_entry(&VoxelKey::ROOT).unwrap();
        assert_eq!(root_entry.key, VoxelKey::ROOT);
        assert_eq!(root_entry.point_count, 107);
    }

    #[test]
    fn test_copc_read_autzen() {
        let copc_points = {
            let file = BufReader::new(File::open("tests/data/autzen.copc.laz").unwrap());
            let mut entry_reader = CopcReader::new(file).unwrap();
            entry_reader
                .query(LodSelection::All, BoundsSelection::All)
                .unwrap()
                .points()
                .collect::<Result<Vec<_>>>()
                .unwrap()
        };
        let laz_points: Vec<_> = Reader::from_path("tests/data/autzen.copc.laz")
            .unwrap()
            .read_all()
            .unwrap()
            .points()
            .collect::<Result<_>>()
            .unwrap();
        assert!(laz_points
            .iter()
            .zip(copc_points)
            .all(|(laz_point, copc_point)| laz_point.eq(&copc_point)));
    }

    #[test]
    fn test_copc_query_autzen() {
        let mut reader = CopcReader::from_path("tests/data/autzen.copc.laz").unwrap();
        let bounds = reader.header().bounds();
        let points = reader
            .query(LodSelection::Level(0), BoundsSelection::Within(bounds))
            .unwrap();
        assert_eq!(points.len(), reader.header().number_of_points() as usize);
        assert!(points.points().all(|point| {
            let point = point.unwrap();
            point.x >= bounds.min.x
                && point.x <= bounds.max.x
                && point.y >= bounds.min.y
                && point.y <= bounds.max.y
                && point.z >= bounds.min.z
                && point.z <= bounds.max.z
        }));
    }

    #[test]
    fn test_copc_query_filters_bounds() {
        let mut reader = CopcReader::from_path("tests/data/autzen.copc.laz").unwrap();
        let mut bounds = reader.header().bounds();
        bounds.max.x = (bounds.min.x + bounds.max.x) / 2.0;
        let points = reader
            .query(LodSelection::Level(0), BoundsSelection::Within(bounds))
            .unwrap();
        assert!(!points.is_empty());
        assert!(points.x().all(|x| x >= bounds.min.x && x <= bounds.max.x));

        let repeated = reader
            .query(LodSelection::Level(0), BoundsSelection::Within(bounds))
            .unwrap();
        assert_eq!(points.raw_bytes(), repeated.raw_bytes());
    }

    #[test]
    fn test_copc_query_rejects_invalid_resolution() {
        let mut reader = CopcReader::from_path("tests/data/autzen.copc.laz").unwrap();
        assert!(reader
            .query(LodSelection::Resolution(0.0), BoundsSelection::All,)
            .is_err());
    }
    #[test]
    fn test_voxel_bounds() {
        let copc_info = CopcInfoVlr {
            center_x: 10.,
            center_y: 10.,
            center_z: 10.,
            halfsize: 5.,
            spacing: 1.,
            root_hier_offset: 0,
            root_hier_size: 0,
            gpstime_minimum: 0.,
            gpstime_maximum: 0.,
            reserved: [0; 11],
        };
        let key = VoxelKey::ROOT.child(3).unwrap();
        let bounds = key.bounds(&copc_info);
        assert_eq!(
            bounds,
            Bounds {
                min: Vector {
                    x: 10.0,
                    y: 10.0,
                    z: 5.0
                },
                max: Vector {
                    x: 15.0,
                    y: 15.0,
                    z: 10.0
                }
            }
        );
    }
}
