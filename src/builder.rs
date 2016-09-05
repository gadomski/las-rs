use std::fs::File;
use std::io::{BufWriter, Read, Seek, Write};
use std::path::Path;

use {Error, Result};
use global_encoding::GlobalEncoding;
use header::Header;
use point::Format;
use reader::Reader;
use utils::{LinearTransform, Triple};
use version::Version;
use vlr::Vlr;
use writer::Writer;

/// Configure a `Writer`.
#[derive(Debug, Default)]
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
    pub system_id: [u8; 32],
    /// The generating software
    pub generating_software: [u8; 32],
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
        header.point_format = builder.point_format;
        header.transforms = builder.transforms;
        header.extra_bytes = builder.extra_bytes;
        header
    }
}
