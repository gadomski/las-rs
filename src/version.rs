use {Error, Result};
use Feature;
use std::fmt;

/// LAS version.
///
/// Defaults to 1.2.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// The major version.
    ///
    /// Should always be 1.
    pub major: u8,
    /// The minor version.
    ///
    /// Should be between 0 and 4.
    pub minor: u8,
}

impl Version {
    /// Creates a new version.
    ///
    /// Doesn't do any checking that its an actual las version
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// let version = Version::new(1, 2);
    /// ```
    pub fn new(major: u8, minor: u8) -> Version {
        Version {
            major: major,
            minor: minor,
        }
    }

    /// Does this version require the point data start signature?
    ///
    /// Only 1.0 does.
    ///
    /// ```
    /// use las::Version;
    /// assert!(Version::new(1, 0).requires_point_data_start_signature());
    /// assert!(!Version::new(1, 1).requires_point_data_start_signature());
    /// ```
    pub fn requires_point_data_start_signature(&self) -> bool {
        self == &Version::new(1, 0)
    }

    /// Returns this version's header size.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert_eq!(227, Version::new(1, 2).header_size());
    /// assert_eq!(235, Version::new(1, 3).header_size());
    /// assert_eq!(375, Version::new(1, 4).header_size());
    /// ```
    pub fn header_size(&self) -> u16 {
        if self <= &Version::new(1, 2) {
            227
        } else if self == &Version::new(1, 3) {
            235
        } else {
            375
        }
    }

    /// Checks whether this version supports the feature, returning an error if not.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// use las::feature::Color;
    /// Version::new(1, 2).verify_support_for::<Color>().unwrap();
    /// assert!(Version::new(1, 0).verify_support_for::<Color>().is_err());
    /// ```
    pub fn verify_support_for<F: Feature>(&self) -> Result<()> {
        if self.supports::<F>() {
            Ok(())
        } else {
            Err(Error::Feature(*self, F::name()))
        }
    }

    /// Checks whether this version supports the feature.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// use las::feature::Color;
    /// assert!(Version::new(1, 2).supports::<Color>());
    /// assert!(!Version::new(1, 0).supports::<Color>());
    /// ```
    pub fn supports<F: Feature>(&self) -> bool {
        F::is_supported_by(*self)
    }
}

impl Default for Version {
    fn default() -> Version {
        Version { major: 1, minor: 2 }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl From<(u8, u8)> for Version {
    fn from((major, minor): (u8, u8)) -> Version {
        Version {
            major: major,
            minor: minor,
        }
    }
}

impl From<Version> for (u8, u8) {
    fn from(version: Version) -> (u8, u8) {
        (version.major, version.minor)
    }
}
