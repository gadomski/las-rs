//! Roundtrip (write-read) tests for supported LAS versions and attributes.

extern crate chrono;
extern crate las;

use las::{Header, Point, Reader, Writer};
use std::io::Cursor;

pub fn roundtrip(header: Header, point: Point, should_succeed: bool) {
    let mut cursor = Cursor::new(Vec::new());
    {
        match Writer::new(&mut cursor, header.clone()).and_then(
            |mut writer| {
                writer.write(point.clone())
            },
        ) {
            Ok(()) => if !should_succeed {
                panic!("Expected write to fail, but it succeeded");
            },
            Err(err) => {
                if should_succeed {
                    panic!("Write error: {}", err)
                } else {
                    return;
                }
            }
        }
    }
    cursor.set_position(0);
    let mut reader = Reader::new(cursor).unwrap();
    let other = reader
        .read()
        .expect("Error when reading the ont point")
        .unwrap();
    assert_eq!(point, other);
    assert_eq!(
        None,
        reader.read().expect("Error when reading past last point")
    );

    let other = reader.header;
    assert_eq!(header.file_source_id, other.file_source_id);
    assert_eq!(header.gps_time_type, other.gps_time_type);
    assert_eq!(header.guid, other.guid);
    assert_eq!(header.version, other.version);
    assert_eq!(header.system_identifier, other.system_identifier);
    assert_eq!(header.generating_software, other.generating_software);
    assert_eq!(header.date, other.date);
    assert_eq!(header.padding, other.padding);
    assert_eq!(header.point_format, other.point_format);
    assert_eq!(header.transforms, other.transforms);
    assert_eq!(point.x, other.bounds.min.x);
    assert_eq!(point.x, other.bounds.max.x);
    assert_eq!(point.y, other.bounds.min.y);
    assert_eq!(point.y, other.bounds.max.y);
    assert_eq!(point.z, other.bounds.min.z);
    assert_eq!(point.z, other.bounds.max.z);
    assert_eq!(1, other.number_of_points);
    if point.return_number > 0 {
        assert_eq!(1, other.number_of_points_by_return[&point.return_number]);
    }
    assert_eq!(header.vlrs, other.vlrs);
}

macro_rules! roundtrip_point {
    ($name:ident, $modify_point:expr) => {
        roundtrip_point!($name, $modify_point, 0);
    };
    ($name:ident, $modify_point:expr, $min_version_minor:expr) => {
        roundtrip_point!($name, $modify_point, $min_version_minor, |_f| {});
    };
    ($name:ident, $modify_point:expr, $min_version_minor:expr, $modify_point_format:expr) => {
        #[test]
        fn $name() {
            use las::{Header, Point, Version};
            use las::point::Format;

            let version = super::version();
            let should_succeed = version >= Version::new(1, $min_version_minor);
            let mut point_format = Format::default();
            $modify_point_format(&mut point_format);
            let mut point = Point::default();
            $modify_point(&mut point);
            let header = Header { version: version, point_format: point_format, ..Default::default() };
            ::roundtrip(header, point, should_succeed);
        }
    };
}

macro_rules! roundtrip_header {
    ($name:ident, $modify_header:expr) => {
        roundtrip_header!($name, $modify_header, 0);
    };
    ($name:ident, $modify_header:expr, $min_version_minor:expr) => {
        #[test]
        fn $name() {
            use las::{Version, Header, Point};

            let version = super::version();
            let should_succeed = version >= Version::new(1, $min_version_minor);
            let mut header = Header { version: version, ..Default::default() };
            $modify_header(&mut header);
            ::roundtrip(header, Point::default(), should_succeed);
        }
    };
}

macro_rules! version {
    ($name:ident, $major:expr, $minor:expr) => {
        mod $name {
            use las::Version;

            fn version() -> Version {
                Version::new($major, $minor)
            }

            mod point {
                use las::Color;
                use las::point::{Classification, ScanDirection};

                roundtrip_point!(xyz, |p: &mut Point| { p.x = 1.; p.y = 2.; p.z = 3.; });
                roundtrip_point!(intensity, |p: &mut Point| p.intensity = 42);
                roundtrip_point!(return_number, |p: &mut Point| p.return_number = 2);
                roundtrip_point!(number_of_returns, |p: &mut Point| p.number_of_returns = 2);
                roundtrip_point!(scan_direction, |p: &mut Point| p.scan_direction = ScanDirection::LeftToRight);
                roundtrip_point!(is_edge_of_flight_line, |p: &mut Point| p.is_edge_of_flight_line = true);
                roundtrip_point!(classification, |p: &mut Point| p.classification = Classification::Ground);
                roundtrip_point!(is_synthetic, |p: &mut Point| p.is_synthetic = true);
                roundtrip_point!(is_key_point, |p: &mut Point| p.is_key_point = true);
                roundtrip_point!(is_withheld, |p: &mut Point| p.is_withheld = true);
                roundtrip_point!(is_overlap, |p: &mut Point| {
                    p.classification = Classification::Unclassified;
                    p.is_overlap = true;
                });
                roundtrip_point!(scanner_channel, |p: &mut Point| {
                    p.scanner_channel = 1;
                    p.gps_time = Some(42.);
                }, 4, |f: &mut Format| f.extend());
                roundtrip_point!(scan_angle_rank, |p: &mut Point| p.scan_angle = 3.);
                roundtrip_point!(user_data, |p: &mut Point| p.user_data = 42);
                roundtrip_point!(point_source_id, |p: &mut Point| p.point_source_id = 42);
                roundtrip_point!(gps_time, |p: &mut Point| p.gps_time = Some(42.), 0, |f: &mut Format| f.has_gps_time = true);
                roundtrip_point!(color, |p: &mut Point| p.color = Some(Color { red: 1, green: 2, blue: 3 }), 2, |f: &mut Format| f.has_color = true);
                // TODO waveform
                roundtrip_point!(nir, |p: &mut Point| {
                    p.color = Some(Color { red: 1, green: 2, blue: 3});
                    p.nir = Some(42);
                    p.gps_time = Some(42.);
                }, 4, |f: &mut Format| {
                    f.extend();
                    f.has_color = true;
                    f.has_nir = true;
                });
                roundtrip_point!(extra_bytes, |p: &mut Point| p.extra_bytes = vec![42], 0, |f: &mut Format| f.extra_bytes = 1);
            }

            mod header {
                use chrono::{Utc, TimeZone};
                use las::GpsTimeType;

                roundtrip_header!(file_source_id, |h: &mut Header| h.file_source_id = 42, 1);
                roundtrip_header!(gps_time_type, |h: &mut Header| h.gps_time_type = GpsTimeType::Standard, 2);
                roundtrip_header!(guid, |h: &mut Header| h.guid = [42; 16]);
                roundtrip_header!(system_identifier, |h: &mut Header| h.system_identifier = "roundtrip test".to_string());
                roundtrip_header!(generating_software, |h: &mut Header| h.generating_software = "roundtrip test".to_string());
                roundtrip_header!(date, |h: &mut Header| h.date = Some(Utc.ymd(2017, 10, 30)));
                roundtrip_header!(padding, |h: &mut Header| h.padding = b"You probably shouldn't do this".to_vec());
                roundtrip_header!(vlr_padding, |h: &mut Header| h.vlr_padding = b"You probably shouldn't do this either".to_vec());
                roundtrip_header!(transforms, |h: &mut Header| {
                    use las::{Transform, Vector};

                    let transform = Transform { scale: 0.1, offset: -1. };
                    h.transforms = Vector {
                        x: transform,
                        y: transform,
                        z: transform,
                    };
                });
                roundtrip_header!(vlrs, |h: &mut Header| h.vlrs = vec![Default::default()]);
                roundtrip_header!(evlrs, |h: &mut Header| {
                    use std::u16;
                    use las::Vlr;

                    let mut vlr = Vlr::default();
                    vlr.is_extended = true;
                    vlr.data = vec![42; u16::MAX as usize + 1];
                    h.vlrs = vec![vlr];
                }, 4);
            }
        }
    }
}

version!(las_1_0, 1, 0);
version!(las_1_1, 1, 1);
version!(las_1_2, 1, 2);
version!(las_1_3, 1, 3);
version!(las_1_4, 1, 4);
