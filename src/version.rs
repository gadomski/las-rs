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

    /// Does this version support file source id?
    ///
    /// Only 1.0 does not.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 0).supports_file_source_id());
    /// assert!(Version::new(1, 1).supports_file_source_id());
    /// ```
    pub fn supports_file_source_id(&self) -> bool {
        self > &Version::new(1, 0)
    }

    /// Does this version support color?
    ///
    /// 1.1 and 1.0 do not.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 0).supports_color());
    /// assert!(!Version::new(1, 1).supports_color());
    /// assert!(Version::new(1, 2).supports_color());
    /// ```
    pub fn supports_color(&self) -> bool {
        self > &Version::new(1, 1)
    }

    /// Does this version support gps standard time?
    ///
    /// 1.1 and 1.0 do not.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 0).supports_gps_standard_time());
    /// assert!(!Version::new(1, 1).supports_gps_standard_time());
    /// assert!(Version::new(1, 2).supports_gps_standard_time());
    /// ```
    pub fn supports_gps_standard_time(&self) -> bool {
        self > &Version::new(1, 1)
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

    /// Returns true if this version has 64 bit support.
    ///
    /// 64 bit support means that the file's number of point records is stored in a 64 bit value,
    /// allowing more points in a las file.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 2).is_64bit());
    /// assert!(Version::new(1, 4).is_64bit());
    /// ```
    pub fn is_64bit(&self) -> bool {
        self >= &Version::new(1, 4)
    }

    /// Returns true if this version supports extended variable length records.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 2).supports_evlrs());
    /// assert!(Version::new(1, 4).supports_evlrs());
    /// ```
    pub fn supports_evlrs(&self) -> bool {
        self >= &Version::new(1, 4)
    }

    /// Returns true if this version supports waveforms.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// assert!(!Version::new(1, 2).supports_waveforms());
    /// assert!(Version::new(1, 3).supports_waveforms());
    /// ```
    pub fn supports_waveforms(&self) -> bool {
        self >= &Version::new(1, 3)
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
