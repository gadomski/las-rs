use std::borrow::ToOwned;
use ::{Vlr, Header};

pub(crate) fn create_laszip_vlr(laszip_vlr_data: Vec<u8>) -> Vlr {
    Vlr{
        user_id: "laszip encoded".to_owned(),
        record_id: 22204,
        description: "http://laszip.org".to_owned(),
        data: laszip_vlr_data
    }
}
pub(crate) fn create_record_schema(header: &Header) -> lazperf::RecordSchema {
    let mut schema = lazperf::RecordSchema::new();
    schema.push_point();
    if header.point_format().has_gps_time {
        schema.push_gpstime();

    }

    if header.point_format().has_color {
        schema.push_rgb();

    }

    if header.point_format().extra_bytes != 0 {
        schema.push_extrabytes(header.point_format().extra_bytes as usize);
    }
    schema
}
