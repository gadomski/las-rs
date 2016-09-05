use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use {Error, Result};
use header::Header;
use reader::Reader;
use vlr::Vlr;
use writer::Writer;

/// Configure a `Writer`.
#[derive(Debug)]
pub struct Builder {
    /// The header that will be used for the new file.
    pub header: Header,
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
        Builder {
            header: Header::default(),
            vlrs: Vec::new(),
        }
    }

    /// Creates a new `Builder` and configures it to match the provided `Reader`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Builder;
    /// use las::Reader;
    /// let reader = Reader::from_path("data/1.0_0.las").unwrap();
    /// let builder = Builder::from_reader(&reader);
    /// ```
    pub fn from_reader<R>(reader: &Reader<R>) -> Builder {
        Builder {
            header: reader.header,
            vlrs: reader.vlrs.clone(),
        }
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

impl Default for Builder {
    fn default() -> Builder {
        Builder::new()
    }
}
