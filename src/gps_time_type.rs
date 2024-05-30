/// The meaning of GPS time in the point records.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GpsTimeType {
    /// GPS Week Time (the same as previous versions of LAS).
    #[default]
    Week,

    /// Standard GPS Time minus 1e9.
    Standard,
}

impl GpsTimeType {
    /// Returns true if this time type is gps standard time.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::GpsTimeType;
    /// assert!(!GpsTimeType::Week.is_standard());
    /// assert!(GpsTimeType::Standard.is_standard());
    /// ```
    pub fn is_standard(&self) -> bool {
        match *self {
            GpsTimeType::Week => false,
            GpsTimeType::Standard => true,
        }
    }
}

impl From<GpsTimeType> for u16 {
    fn from(gps_time_type: GpsTimeType) -> u16 {
        match gps_time_type {
            GpsTimeType::Week => 0,
            GpsTimeType::Standard => 1,
        }
    }
}

impl From<u16> for GpsTimeType {
    fn from(n: u16) -> GpsTimeType {
        match n & 1 {
            0 => GpsTimeType::Week,
            1 => GpsTimeType::Standard,
            _ => unreachable!(),
        }
    }
}
