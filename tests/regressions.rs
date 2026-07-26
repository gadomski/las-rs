use las::{Builder, Reader, Writer};
use tempfile::NamedTempFile;

#[test]
fn issue_136() {
    let mut reader = Reader::from_path("tests/data/autzen.las").unwrap();
    let points: Vec<las::Point> = reader
        .read_all()
        .unwrap()
        .points()
        .collect::<las::Result<_>>()
        .unwrap();

    let mut builder = Builder::from((1, 4));
    builder.point_format = las::point::Format::new(1).unwrap();
    let header = builder.into_header().unwrap();

    let tempfile = NamedTempFile::new().unwrap();
    let file_name = tempfile.path().to_str().unwrap().to_string();
    {
        let mut writer = Writer::from_path(&file_name, header).unwrap();
        for point in points {
            writer.write_point(point).unwrap();
        }
    }

    let reader = Reader::from_path(file_name).unwrap();
    assert_eq!(reader.header().number_of_points(), 106);
}

// Regression tests for the untrusted `number_of_point_records` / LargeFile
// count preallocation class (c.f. issue #125 "OOM crashes in COPC LAZ files").
//
// The header's point count is attacker-controlled and was previously honoured
// verbatim by `Reader::read_all`, which reserved `count * record_len` bytes
// before reading a single point. A sub-kibibyte file with a crafted count
// could therefore force a multi-gibibyte allocation, a `capacity overflow`
// panic, or — on LAS 1.4 — an `attempt to multiply with overflow` panic
// during `Reader::new`. These tests assert the reader now returns a
// structured `las::Error` instead of panicking or attempting the allocation.
mod untrusted_point_count {
    use byteorder::{LittleEndian as LE, WriteBytesExt};
    use las::Reader;
    use std::io::Cursor;

    fn write_common(
        buf: &mut Vec<u8>,
        version_major: u8,
        version_minor: u8,
        header_size: u16,
        offset_to_point_data: u32,
        point_data_record_format: u8,
        point_data_record_length: u16,
        number_of_point_records: u32,
    ) {
        buf.extend_from_slice(b"LASF");
        buf.write_u16::<LE>(0).unwrap(); // file_source_id
        buf.write_u16::<LE>(0).unwrap(); // global_encoding
        buf.extend_from_slice(&[0u8; 16]); // guid
        buf.write_u8(version_major).unwrap();
        buf.write_u8(version_minor).unwrap();
        buf.extend_from_slice(&[0u8; 32]); // system_identifier
        buf.extend_from_slice(&[0u8; 32]); // generating_software
        buf.write_u16::<LE>(0).unwrap(); // file_creation_day_of_year
        buf.write_u16::<LE>(2026).unwrap(); // file_creation_year
        buf.write_u16::<LE>(header_size).unwrap();
        buf.write_u32::<LE>(offset_to_point_data).unwrap();
        buf.write_u32::<LE>(0).unwrap(); // number_of_variable_length_records
        buf.write_u8(point_data_record_format).unwrap();
        buf.write_u16::<LE>(point_data_record_length).unwrap();
        buf.write_u32::<LE>(number_of_point_records).unwrap();
        for _ in 0..5 {
            buf.write_u32::<LE>(0).unwrap(); // number_of_points_by_return
        }
        for _ in 0..12 {
            buf.write_f64::<LE>(0.0).unwrap(); // scales + offsets + min/max
        }
    }

    fn las12_header(count: u32, record_len: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_common(&mut buf, 1, 2, 227, 227, 0, record_len, count);
        assert_eq!(buf.len(), 227);
        buf
    }

    // LAS 1.4 LargeFile: legacy u32 count = 0 forces fallback to the u64
    // LargeFile field. 227 (common) + 8 (waveform) + 12 (evlr) + 128 = 375.
    fn las14_large_file_header(count: u64, record_len: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        write_common(&mut buf, 1, 4, 375, 375, 0, record_len, 0);
        buf.write_u64::<LE>(0).unwrap(); // start_of_waveform_data_packet_record
        buf.write_u64::<LE>(0).unwrap(); // start_of_first_evlr
        buf.write_u32::<LE>(0).unwrap(); // number_of_evlrs
        buf.write_u64::<LE>(count).unwrap();
        for _ in 0..15 {
            buf.write_u64::<LE>(0).unwrap(); // number_of_points_by_return
        }
        assert_eq!(buf.len(), 375);
        buf
    }

    /// A LAS 1.2 file whose header claims ~1.07e9 points but whose body is
    /// only 64 bytes. Previously this made `read_all` try to reserve ~20 GiB
    /// before reading; now it must return a structured error.
    #[test]
    fn las12_huge_count_short_body_errors_instead_of_allocating() {
        let mut data = las12_header(0x4000_0000, 20);
        data.extend_from_slice(&[0u8; 64]);
        let mut reader = Reader::new(Cursor::new(data)).unwrap();
        let err = reader.read_all().unwrap_err();
        assert!(
            matches!(err, las::Error::PointRecordLengthOverflow { .. }),
            "expected PointRecordLengthOverflow, got {err:?}"
        );
    }

    /// A LAS 1.4 file whose LargeFile count is `u64::MAX`: the
    // `count * record_len` product overflows `u64`. Previously this panicked
    // with `attempt to multiply with overflow` (debug) during `Reader::new`,
    // or `capacity overflow` (release) inside `read_all`; now `Reader::new`
    // must return the structured error.
    #[test]
    fn las14_large_file_overflow_count_errors_in_reader_new() {
        let mut data = las14_large_file_header(u64::MAX, 20);
        data.extend_from_slice(&[0u8; 64]);
        let err = match Reader::new(Cursor::new(data)) {
            Ok(_) => panic!("expected Reader::new to reject overflowing large-file count"),
            Err(e) => e,
        };
        assert!(
            matches!(err, las::Error::PointRecordLengthOverflow { .. }),
            "expected PointRecordLengthOverflow, got {err:?}"
        );
    }

    /// Positive control: a well-formed LAS 1.2 file whose header count matches
    /// its body must still read end-to-end (guards against over-eager rejection).
    #[test]
    fn valid_short_las12_round_trips() {
        let n: u32 = 3;
        let mut data = las12_header(n, 20);
        for i in 0..n {
            let mut rec = vec![0u8; 20];
            rec[0..4].copy_from_slice(&(i as i32).to_le_bytes()); // x
            data.extend_from_slice(&rec);
        }
        let mut reader = Reader::new(Cursor::new(data)).unwrap();
        assert_eq!(reader.header().number_of_points(), u64::from(n));
        let pd = reader.read_all().unwrap();
        assert_eq!(pd.len(), usize::try_from(n).unwrap());
    }
}
