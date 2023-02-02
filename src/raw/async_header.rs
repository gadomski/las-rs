//! Raw file metadata.

use crate::feature::{Evlrs, LargeFiles, Waveforms};
use crate::raw::LASF;
use crate::raw::{header::Evlr, header::LargeFile, Header};
use crate::{reader, Result, Version};
// use byteorder::{ByteOrder, LittleEndian};
use byteorder_async::{LittleEndian, ReaderToByteOrder};
use futures::future::{AndThen, MapOk};
use futures::io::{AsyncRead, AsyncReadExt, ReadExact};
use futures::task::{Context, Poll};
use futures::{Future, TryFuture, TryFutureExt};
use std::future::IntoFuture;
use std::marker::{PhantomData, Unpin};
use std::pin::Pin;

impl Header {
    /// Reads a raw header from a `AsyncRead`.
    ///
    /// Generally very permissive, but will throw an error if a couple of things are true:
    ///
    /// - The file signature is not exactly "LASF".
    /// - The point data format is not recognized. Note that version mismatches *are* allowed (e.g.
    /// color points for las 1.1).
    /// - The point data record length is less than the minimum length of the point data format.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs::File;
    /// use las::raw::Header;
    /// let mut file = File::open("tests/data/autzen.las").unwrap();
    /// let header = Header::read_from(&mut file).unwrap();
    /// ```
    pub async fn read_from_async<R: AsyncRead + Unpin>(mut the_read: R) -> Result<Header> {
        use crate::header::Error;
        use crate::utils;

        let mut read = the_read.byte_order();

        let mut header = Header::default();

        read.read_exact(&mut header.file_signature).await?;
        if header.file_signature != LASF {
            return Err(Error::FileSignature(header.file_signature).into());
        }
        header.file_source_id = read.read_u16::<LittleEndian>().await?;
        header.global_encoding = read.read_u16::<LittleEndian>().await?;
        read.read_exact(&mut header.guid).await?;
        let version_major = read.read_u8().await?;
        let version_minor = read.read_u8().await?;
        header.version = Version::new(version_major, version_minor);
        read.read_exact(&mut header.system_identifier).await?;
        read.read_exact(&mut header.generating_software).await?;
        header.file_creation_day_of_year = read.read_u16::<LittleEndian>().await?;
        header.file_creation_year = read.read_u16::<LittleEndian>().await?;
        header.header_size = read.read_u16::<LittleEndian>().await?;
        header.offset_to_point_data = read.read_u32::<LittleEndian>().await?;
        header.number_of_variable_length_records = read.read_u32::<LittleEndian>().await?;
        header.point_data_record_format = read.read_u8().await?;
        header.point_data_record_length = read.read_u16::<LittleEndian>().await?;
        header.number_of_point_records = read.read_u32::<LittleEndian>().await?;
        for n in &mut header.number_of_points_by_return {
            *n = read.read_u32::<LittleEndian>().await?;
        }
        header.x_scale_factor = read.read_f64::<LittleEndian>().await?;
        header.y_scale_factor = read.read_f64::<LittleEndian>().await?;
        header.z_scale_factor = read.read_f64::<LittleEndian>().await?;
        header.x_offset = read.read_f64::<LittleEndian>().await?;
        header.y_offset = read.read_f64::<LittleEndian>().await?;
        header.z_offset = read.read_f64::<LittleEndian>().await?;
        header.max_x = read.read_f64::<LittleEndian>().await?;
        header.min_x = read.read_f64::<LittleEndian>().await?;
        header.max_y = read.read_f64::<LittleEndian>().await?;
        header.min_y = read.read_f64::<LittleEndian>().await?;
        header.max_z = read.read_f64::<LittleEndian>().await?;
        header.min_z = read.read_f64::<LittleEndian>().await?;
        header.start_of_waveform_data_packet_record = if header.version.supports::<Waveforms>() {
            utils::some_or_none_if_zero(read.read_u64::<LittleEndian>().await?)
        } else {
            None
        };
        header.evlr = if header.version.supports::<Evlrs>() {
            // I'm too tired to fight with this
            // Copy paste for the rescue
            Evlr {
                start_of_first_evlr: read.read_u64::<LittleEndian>().await?,
                number_of_evlrs: read.read_u32::<LittleEndian>().await?,
            }
            .into_option()
        } else {
            None
        };
        header.large_file = if header.version.supports::<LargeFiles>() {
            let number_of_point_records = read.read_u64::<LittleEndian>().await?;
            let mut number_of_points_by_return = [0; 15];
            for n in &mut number_of_points_by_return {
                *n = read.read_u64::<LittleEndian>().await?
            }
            Some(LargeFile {
                number_of_point_records,
                number_of_points_by_return,
            })
        } else {
            None
        };
        header.padding = if header.header_size > header.version.header_size() {
            let mut bytes = vec![0; (header.header_size - header.version.header_size()) as usize];
            read.read_exact(&mut bytes).await?;
            bytes
        } else {
            Vec::new()
        };
        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    async fn write_read(header: Header) -> Result<()> {
        let mut cursor = Cursor::new(Vec::new());
        header.write_to(&mut cursor).unwrap();
        cursor.set_position(0);

        let async_cursor = futures::io::Cursor::new(cursor.into_inner());
        Header::read_from_async(async_cursor).await?;
        Ok(())
    }

    #[test]
    fn invalid_file_signature() {
        let header = Header {
            file_signature: *b"ABCD",
            ..Default::default()
        };

        assert!(futures::executor::block_on(async {
            write_read(header).await.is_err()
        }));
    }

    macro_rules! roundtrip {
        ($name:ident, $minor:expr) => {
            mod $name {
                #[test]
                fn roundtrip() {
                    use super::*;
                    use std::io::Cursor;

                    let version = Version::new(1, $minor);
                    let mut header = Header {
                        version,
                        ..Default::default()
                    };
                    if version.minor == 4 {
                        header.large_file = Some(LargeFile::default());
                    }
                    let mut cursor = Cursor::new(Vec::new());
                    header.write_to(&mut cursor).unwrap();
                    cursor.set_position(0);

                    let async_cursor = futures::io::Cursor::new(cursor.into_inner());
                    assert_eq!(
                        header,
                        futures::executor::block_on(async {
                            Header::read_from_async(async_cursor).await.unwrap()
                        })
                    );
                }
            }
        };
    }

    roundtrip!(las_1_0, 0);
    roundtrip!(las_1_1, 1);
    roundtrip!(las_1_2, 2);
    roundtrip!(las_1_3, 3);
    roundtrip!(las_1_4, 4);
}
