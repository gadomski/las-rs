/// The meaning of GPS time in the point records.
#[derive(Clone, Copy, Debug)]
pub enum GpsTimeType {
    /// GPS Week Time (the same as previous versions of LAS).
    Week,
    /// Standard GPS Time minu 1e9.
    Standard,
}
