//! Roundtrip (write-read) tests for supported LAS versions and attributes.

extern crate chrono;
extern crate las;
extern crate uuid;

use std::io::Cursor;

use las::{Builder, Point, Read, Reader, Write, Writer};

pub fn roundtrip(builder: Builder, point: &Point, should_succeed: bool) {
    let header = if should_succeed {
        builder.into_header().unwrap()
    } else {
        assert!(builder.into_header().is_err());
        return;
    };
    let mut writer = Writer::new(Cursor::new(Vec::new()), header).unwrap();
    writer.write(point.clone()).unwrap();
    let header = writer.header().clone();
    let mut reader = Reader::new(writer.into_inner().unwrap()).unwrap();
    assert_eq!(*point, reader.read().unwrap().unwrap());
    assert!(reader.read().is_none());
    assert_eq!(header, *reader.header());
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
            use las::{point::Format, Builder, Point, Version};

            let version = super::version();
            let should_succeed = version >= Version::new(1, $min_version_minor);
            let mut point_format = Format::default();
            $modify_point_format(&mut point_format);
            let mut point = Point::default();
            $modify_point(&mut point);
            let mut builder = Builder::from(version);
            builder.point_format = point_format;
            crate::roundtrip(builder, &point, should_succeed);
        }
    };
}

macro_rules! roundtrip_builder {
    ($name:ident, $modify_builder:expr) => {
        roundtrip_builder!($name, $modify_builder, 0);
    };
    ($name:ident, $modify_builder:expr, $min_version_minor:expr) => {
        #[test]
        fn $name() {
            use las::{Builder, Point, Version};

            let version = super::version();
            let should_succeed = version >= Version::new(1, $min_version_minor);
            let mut builder = Builder::from(version);
            $modify_builder(&mut builder);
            crate::roundtrip(builder, &Point::default(), should_succeed);
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
                use las::{
                    point::{Classification, ScanDirection},
                    Color,
                };

                roundtrip_point!(xyz, |p: &mut Point| {
                    p.x = 1.;
                    p.y = 2.;
                    p.z = 3.;
                });
                roundtrip_point!(intensity, |p: &mut Point| p.intensity = 42);
                roundtrip_point!(return_number, |p: &mut Point| p.return_number = 2);
                roundtrip_point!(number_of_returns, |p: &mut Point| p.number_of_returns = 2);
                roundtrip_point!(scan_direction, |p: &mut Point| p.scan_direction =
                    ScanDirection::LeftToRight);
                roundtrip_point!(is_edge_of_flight_line, |p: &mut Point| p
                    .is_edge_of_flight_line =
                    true);
                roundtrip_point!(classification, |p: &mut Point| p.classification =
                    Classification::Ground);
                roundtrip_point!(is_synthetic, |p: &mut Point| p.is_synthetic = true);
                roundtrip_point!(is_key_point, |p: &mut Point| p.is_key_point = true);
                roundtrip_point!(is_withheld, |p: &mut Point| p.is_withheld = true);
                roundtrip_point!(is_overlap, |p: &mut Point| {
                    p.classification = Classification::Unclassified;
                    p.is_overlap = true;
                });
                roundtrip_point!(
                    scanner_channel,
                    |p: &mut Point| {
                        p.scanner_channel = 1;
                        p.gps_time = Some(42.);
                    },
                    4,
                    |f: &mut Format| f.extend()
                );
                roundtrip_point!(scan_angle_rank, |p: &mut Point| p.scan_angle = 3.);
                roundtrip_point!(user_data, |p: &mut Point| p.user_data = 42);
                roundtrip_point!(point_source_id, |p: &mut Point| p.point_source_id = 42);
                roundtrip_point!(
                    gps_time,
                    |p: &mut Point| p.gps_time = Some(42.),
                    0,
                    |f: &mut Format| f.has_gps_time = true
                );
                roundtrip_point!(
                    color,
                    |p: &mut Point| p.color = Some(Color {
                        red: 1,
                        green: 2,
                        blue: 3
                    }),
                    2,
                    |f: &mut Format| f.has_color = true
                );
                // TODO waveform
                roundtrip_point!(
                    nir,
                    |p: &mut Point| {
                        p.color = Some(Color {
                            red: 1,
                            green: 2,
                            blue: 3,
                        });
                        p.nir = Some(42);
                        p.gps_time = Some(42.);
                    },
                    4,
                    |f: &mut Format| {
                        f.extend();
                        f.has_color = true;
                        f.has_nir = true;
                    }
                );
                roundtrip_point!(
                    extra_bytes,
                    |p: &mut Point| p.extra_bytes = vec![42],
                    0,
                    |f: &mut Format| f.extra_bytes = 1
                );
            }

            mod builder {
                use chrono::NaiveDate;
                use las::{GpsTimeType, Vlr};
                use uuid::Uuid;

                roundtrip_builder!(file_source_id, |b: &mut Builder| b.file_source_id = 42, 1);
                roundtrip_builder!(
                    gps_time_type,
                    |b: &mut Builder| b.gps_time_type = GpsTimeType::Standard,
                    2
                );
                roundtrip_builder!(
                    has_synthetic_return_numbers,
                    |b: &mut Builder| b.has_synthetic_return_numbers = true,
                    3
                );
                roundtrip_builder!(guid, |b: &mut Builder| b.guid = Uuid::from_bytes([42; 16]));
                roundtrip_builder!(system_identifier, |b: &mut Builder| b.system_identifier =
                    "roundtrip test".to_string());
                roundtrip_builder!(generating_software, |b: &mut Builder| b
                    .generating_software =
                    "roundtrip test".to_string());
                roundtrip_builder!(date, |b: &mut Builder| b.date =
                    NaiveDate::from_ymd_opt(2017, 10, 30));
                roundtrip_builder!(transforms, |b: &mut Builder| {
                    use las::{Transform, Vector};

                    let transform = Transform {
                        scale: 0.1,
                        offset: -1.,
                    };
                    b.transforms = Vector {
                        x: transform,
                        y: transform,
                        z: transform,
                    };
                });
                roundtrip_builder!(vlrs, |b: &mut Builder| b.vlrs.push(Default::default()));
                roundtrip_builder!(
                    evlrs,
                    |b: &mut Builder| {
                        let mut vlr = Vlr::default();
                        vlr.data = vec![42; ::std::u16::MAX as usize + 1];
                        b.evlrs.push(vlr);
                    },
                    4
                );
                roundtrip_builder!(padding, |b: &mut Builder| b.padding =
                    b"You probably shouldn't do this".to_vec());
                roundtrip_builder!(vlr_padding, |b: &mut Builder| b.vlr_padding =
                    b"You probably shouldn't do this either".to_vec());
                roundtrip_builder!(
                    point_padding,
                    |b: &mut Builder| {
                        b.point_padding = vec![42];
                        b.evlrs.push(Vlr::default());
                    },
                    4
                );
            }
        }
    };
}

version!(las_1_0, 1, 0);
version!(las_1_1, 1, 1);
version!(las_1_2, 1, 2);
version!(las_1_3, 1, 3);
version!(las_1_4, 1, 4);
