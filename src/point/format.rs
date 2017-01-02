use std::fmt;

/// A point format describes the attributes associated with the point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Format(u8);

impl Format {
    /// Returns true if this format has GPS time attached to each point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert!(!Format::from(0).has_gps_time());
    /// assert!(Format::from(1).has_gps_time());
    /// ```
    pub fn has_gps_time(&self) -> bool {
        self.0 % 2 == 1
    }

    /// Returns true if this point format has color.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert!(!Format::from(0).has_color());
    /// assert!(Format::from(2).has_color());
    /// ```
    pub fn has_color(&self) -> bool {
        self.0 / 2 == 1
    }

    /// Returns the length of this point format.
    ///
    /// Does not include any extra bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::point::Format;
    /// assert_eq!(20, Format::from(0).len());
    /// ```
    pub fn len(&self) -> u16 {
        let mut len = 20;
        if self.has_gps_time() {
            len += 8;
        }
        if self.has_color() {
            len += 6;
        }
        len
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

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "point format {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_gps_time() {
        assert!(!Format::from(0).has_gps_time());
        assert!(Format::from(1).has_gps_time());
        assert!(!Format::from(2).has_gps_time());
        assert!(Format::from(3).has_gps_time());
    }

    #[test]
    fn has_color() {
        assert!(!Format::from(0).has_color());
        assert!(!Format::from(1).has_color());
        assert!(Format::from(2).has_color());
        assert!(Format::from(3).has_color());
    }

    #[test]
    fn len() {
        assert_eq!(20, Format::from(0).len());
        assert_eq!(28, Format::from(1).len());
        assert_eq!(26, Format::from(2).len());
        assert_eq!(34, Format::from(3).len());
    }
}
