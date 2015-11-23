pub fn scale(record: i32, scale: f64, offset: f64) -> f64 {
    (record as f64) * scale + offset
}

pub fn descale(coordinate: f64, scale: f64, offset: f64) -> i32 {
    ((coordinate - offset) / scale) as i32
}
