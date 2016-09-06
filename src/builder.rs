use std::fs::File;
use std::io::{BufWriter, Read, Seek, Write};
use std::path::Path;

use {Error, Result};
use global_encoding::GlobalEncoding;
use header::Header;
use point::Format;
use reader::Reader;
use utils::{FromLasStr, LinearTransform, ToLasStr, Triple};
use version::Version;
use vlr::Vlr;
use writer::Writer;

/// Configure a `Writer`.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
/// # use las::Builder;
/// let mut builder = Builder::new();
/// builder.vlrs.push(Default::default());
/// builder.set_generating_software("las-rs example");
/// let writer = builder.writer(Cursor::new(Vec::new())).unwrap();
/// ```
#[derive(Debug)]
pub struct Builder {
    /// The file source id.
    pub file_source_id: u16,
    /// The global encoding for the new file.
    pub global_encoding: GlobalEncoding,
    /// The project id number (GUID).
    pub project_id: [u8; 16],
    /// The LAS version.
    pub version: Version,
    /// The system ID.
    system_id: [u8; 32],
    /// The generating software
    generating_software: [u8; 32],
    /// The point format.
    pub point_format: Format,
    /// The linear transformations for each dimension.
    pub transforms: Triple<LinearTransform>,
    /// The extra bytes on each point.
    pub extra_bytes: u16,
    /// The VLRs that will be included in the new file.
    pub vlrs: Vec<Vlr>,
}

impl Builder {
    /// Creates a new `Builder`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let builder = Builder::new();
    /// ```
    pub fn new() -> Builder {
        Builder { ..Default::default() }
    }

    /// Returns the system id as a string.
    pub fn system_id(&self) -> Result<&str> {
        self.system_id.to_las_str_strict()
    }

    /// Sets the system id from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let mut builder = Builder::new();
    /// builder.set_system_id("las-rs example").unwrap();
    /// assert_eq!("las-rs example", builder.system_id().unwrap());
    /// ```
    pub fn set_system_id(&mut self, system_id: &str) -> Result<()> {
        self.system_id.from_las_str(system_id)
    }

    /// Returns the generating software as a string.
    pub fn generating_software(&self) -> Result<&str> {
        self.generating_software.to_las_str_strict()
    }

    /// Sets the generating software from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let mut builder = Builder::new();
    /// builder.set_generating_software("las-rs example").unwrap();
    /// assert_eq!("las-rs example", builder.generating_software().unwrap());
    /// ```
    pub fn set_generating_software(&mut self, generating_software: &str) -> Result<()> {
        self.generating_software.from_las_str(generating_software)
    }

    /// Creates a `Writer`.
    ///
    /// This method does *not* consume the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Cursor;
    /// # use las::Builder;
    /// let writer = Builder::new().writer(Cursor::new(Vec::new())).unwrap();
    /// ```
    pub fn writer<W: Seek + Write>(&self, write: W) -> Result<Writer<W>> {
        Writer::new(self, write)
    }

    /// Creates a `Writer` that will write out data to the path.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// let writer = Builder::new().writer_from_path("/dev/null").unwrap();
    /// ```
    pub fn writer_from_path<P: AsRef<Path>>(&self, path: P) -> Result<Writer<BufWriter<File>>> {
        File::create(path).map_err(Error::from).and_then(|f| self.writer(BufWriter::new(f)))
    }
}

impl<'a, R: Read + Seek> From<&'a Reader<R>> for Builder {
    fn from(reader: &'a Reader<R>) -> Builder {
        Builder {
            file_source_id: reader.header.file_source_id,
            global_encoding: reader.header.global_encoding,
            project_id: reader.header.project_id,
            version: reader.header.version,
            system_id: reader.header.system_id,
            generating_software: reader.header.generating_software,
            point_format: reader.header.point_format,
            transforms: reader.header.transforms,
            extra_bytes: reader.header.extra_bytes,
            vlrs: reader.vlrs.clone(),
        }
    }
}

impl<'a> From<&'a Builder> for Header {
    fn from(builder: &'a Builder) -> Header {
        let mut header: Header = Default::default();
        header.file_source_id = builder.file_source_id;
        header.global_encoding = builder.global_encoding;
        header.project_id = builder.project_id;
        header.version = builder.version;
        header.system_id = builder.system_id;
        header.generating_software = builder.generating_software;
        header.offset_to_point_data = builder.vlrs
            .iter()
            .fold(header.header_size as u32, |acc, vlr| acc + vlr.len());
        header.num_vlrs = builder.vlrs.len() as u32;
        header.point_format = builder.point_format;
        header.transforms = builder.transforms;
        header.extra_bytes = builder.extra_bytes;
        header
    }
}

impl Default for Builder {
    fn default() -> Builder {
        let mut system_id = [0; 32];
        system_id.from_las_str("las-rs").unwrap();
        let mut generating_software = [0; 32];
        generating_software.from_las_str(&format!("las-rs {}", env!("CARGO_PKG_VERSION"))).unwrap();
        Builder {
            file_source_id: Default::default(),
            global_encoding: Default::default(),
            project_id: Default::default(),
            version: Default::default(),
            system_id: system_id,
            generating_software: generating_software,
            point_format: Default::default(),
            transforms: Default::default(),
            extra_bytes: Default::default(),
            vlrs: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use header::Header;

    #[test]
    fn offset_to_point_data() {
        let mut builder = Builder::new();
        let header: Header = (&builder).into();
        assert_eq!(227, header.offset_to_point_data);
        builder.vlrs.push(Default::default());
        let header: Header = (&builder).into();
        assert_eq!(281, header.offset_to_point_data);
    }

    #[test]
    fn num_vlrs() {
        let mut builder = Builder::new();
        builder.vlrs.push(Default::default());
        let header: Header = (&builder).into();
        assert_eq!(1, header.num_vlrs);
    }
}
