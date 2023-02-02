//! Implementation for Vlr::read_from_async
use crate::raw::vlr::RecordLength;
use crate::raw::Vlr;
use crate::Result;
use byteorder_async::ReaderToByteOrder;
use futures::io::AsyncRead;

impl Vlr {
    /// Reads a raw VLR or EVLR.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Seek, SeekFrom};
    /// use std::fs::File;
    /// use las::raw::Vlr;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// file.seek(SeekFrom::Start(227));
    /// // If the second parameter were true, it would be read as an extended vlr.
    /// let vlr = Vlr::read_from(file, false).unwrap();
    /// ```
    #[allow(clippy::field_reassign_with_default)]
    pub async fn read_from_async<R: AsyncRead + Unpin>(mut read: R, extended: bool) -> Result<Vlr> {
        use byteorder_async::LittleEndian;

        let mut read = read.byte_order();

        let mut vlr = Vlr::default();
        vlr.reserved = read.read_u16::<LittleEndian>().await?;
        read.read_exact(&mut vlr.user_id).await?;
        vlr.record_id = read.read_u16::<LittleEndian>().await?;
        vlr.record_length_after_header = if extended {
            RecordLength::Evlr(read.read_u64::<LittleEndian>().await?)
        } else {
            RecordLength::Vlr(read.read_u16::<LittleEndian>().await?)
        };
        read.read_exact(&mut vlr.description).await?;
        vlr.data
            .resize(usize::from(vlr.record_length_after_header), 0);
        read.read_exact(&mut vlr.data).await?;
        Ok(vlr)
    }
}
