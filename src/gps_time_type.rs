/// The meaning of GPS time in the point records.
#[derive(Clone, Copy, Debug)]
pub enum GpsTimeType {
    /// GPS Week Time (the same as previous versions of LAS).
    Week,
    /// Standard GPS Time minu 1e9.
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
