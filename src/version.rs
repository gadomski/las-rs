use std::fmt;

/// LAS version.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Version {
    /// The major version.
    ///
    /// For now, always 1.
    pub major: u8,
    /// The minor version.
    pub minor: u8,
}

impl Version {
    /// Creates a new version.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Version;
    /// let version = Version::new(1, 2);
    /// ```
    pub fn new(major: u8, minor: u8) -> Version {
        Version {
            major: major,
            minor: minor,
        }
    }

    /// Does this version have a file source id in the header?
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Version;
    /// assert!(!Version::new(1, 0).has_file_source_id());
    /// assert!(Version::new(1, 1).has_file_source_id());
    /// ```
    pub fn has_file_source_id(&self) -> bool {
        !(self.major == 1 && self.minor == 0)
    }

    /// Does this version have a global encoding field?
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Version;
    /// assert!(!Version::new(1, 0).has_global_encoding());
    /// assert!(!Version::new(1, 1).has_global_encoding());
    /// assert!(Version::new(1, 2).has_global_encoding());
    /// ```
    pub fn has_global_encoding(&self) -> bool {
        !(self.major == 1 && self.minor < 2)
    }

    /// Is the classification field for this version mandatory?
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Version;
    /// assert!(!Version::new(1, 0).has_mandatory_classification());
    /// assert!(Version::new(1, 1).has_mandatory_classification());
    /// ```
    pub fn has_mandatory_classification(&self) -> bool {
        !(self.major == 1 && self.minor == 0)
    }
}

impl Default for Version {
    fn default() -> Version {
        Version::new(1, 2)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_has_file_source_id() {
        assert!(!Version::new(1, 0).has_file_source_id());
        assert!(Version::new(1, 1).has_file_source_id());
        assert!(Version::new(1, 2).has_file_source_id());
    }

    #[test]
    fn version_has_global_encoding() {
        assert!(!Version::new(1, 0).has_global_encoding());
        assert!(!Version::new(1, 1).has_global_encoding());
        assert!(Version::new(1, 2).has_global_encoding());
    }

    #[test]
    fn version_has_mandatory_classification() {
        assert!(!Version::new(1, 0).has_mandatory_classification());
        assert!(Version::new(1, 1).has_mandatory_classification());
        assert!(Version::new(1, 2).has_mandatory_classification());
    }
}
