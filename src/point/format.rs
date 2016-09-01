/// Point record format type.
#[derive(Clone, Copy, Debug, Default)]
pub struct Format(u8);

impl Format {
    /// Does this point format have a gps_time field?
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert!(!Format::from(0).has_gps_time());
    /// assert!(Format::from(1).has_gps_time());
    /// ```
    pub fn has_gps_time(&self) -> bool {
        match self.0 {
            0 | 2 => false,
            1 | 3 => true,
            _ => {
                panic!("Don't know whether this point format has gps time: {:?}",
                       self)
            }
        }
    }

    /// Does this point format have color fields?
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert!(!Format::from(0).has_gps_time());
    /// assert!(Format::from(2).has_color());
    /// ```
    pub fn has_color(&self) -> bool {
        match self.0 {
            0 | 1 => false,
            2 | 3 => true,
            _ => panic!("Don't know whether this point format has color: {:?}", self),
        }
    }

    /// Returns true if this point format is supported.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert!(Format::from(0).is_supported());
    /// assert!(!Format::from(99).is_supported());
    /// ```
    pub fn is_supported(&self) -> bool {
        self.0 <= 3
    }

    /// The length of a standard point in this format.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert_eq!(20, Format::from(0).record_length());
    /// ```
    pub fn record_length(&self) -> u16 {
        let mut length = 20;
        if self.has_gps_time() {
            length += 8;
        }
        if self.has_color() {
            length += 6;
        }
        length
    }
}

impl From<u8> for Format {
    fn from(n: u8) -> Format {
        Format(n)
    }
}

impl From<Format> for u8 {
    fn from(format: Format) -> u8 {
        format.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_format_has_gps_time() {
        assert!(!Format::from(0).has_gps_time());
        assert!(Format::from(1).has_gps_time());
        assert!(!Format::from(2).has_gps_time());
        assert!(Format::from(3).has_gps_time());
    }

    #[test]
    fn point_format_has_color() {
        assert!(!Format::from(0).has_color());
        assert!(!Format::from(1).has_color());
        assert!(Format::from(2).has_color());
        assert!(Format::from(3).has_color());
    }

    #[test]
    fn point_format_is_supported() {
        assert!(Format::from(0).is_supported());
        assert!(Format::from(1).is_supported());
        assert!(Format::from(2).is_supported());
        assert!(Format::from(3).is_supported());
        assert!(!Format::from(4).is_supported());
    }

    #[test]
    fn point_format_record_length() {
        assert_eq!(20, Format::from(0).record_length());
        assert_eq!(28, Format::from(1).record_length());
        assert_eq!(26, Format::from(2).record_length());
        assert_eq!(34, Format::from(3).record_length());
    }
}
