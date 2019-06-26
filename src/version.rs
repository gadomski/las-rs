use point::Format;
use std::fmt;
use Feature;
use {Error, Result};

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
    /// use las::feature::Waveforms;
    /// Version::new(1, 4).verify_support_for::<Waveforms>().unwrap();
    /// assert!(Version::new(1, 2).verify_support_for::<Waveforms>().is_err());
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
    /// use las::feature::Waveforms;
    /// assert!(Version::new(1, 4).supports::<Waveforms>());
    /// assert!(!Version::new(1, 2).supports::<Waveforms>());
    /// ```
    pub fn supports<F: Feature>(&self) -> bool {
        F::is_supported_by(*self)
    }

    /// Checks whether this version supports the given point format.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Version;
    /// use las::point::Format;
    /// let las_1_2 = Version::new(1, 2);
    /// let las_1_4 = Version::new(1, 4);
    /// assert!(las_1_2.supports_point_format(Format::new(3).unwrap()));
    /// assert!(!las_1_2.supports_point_format(Format::new(4).unwrap()));
    /// assert!(las_1_4.supports_point_format(Format::new(4).unwrap()));
    /// ```
    pub fn supports_point_format(&self, format: Format) -> bool {
        if self.major != 1 {
            return false;
        }
        match self.minor {
            0 | 1 => {
                !(format.has_color || format.is_extended || format.has_waveform || format.has_nir)
            }
            2 => !(format.is_extended || format.has_waveform || format.has_nir),
            3 => !(format.is_extended || format.has_nir),
            4 => true,
            _ => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! version {
        ($name:ident, $major:expr, $minor:expr, $supports:expr, $max_point_format:expr) => {
            mod $name {
                use super::*;
                use feature::*;

                #[test]
                fn features() {
                    let version = Version::new($major, $minor);
                    assert_eq!($supports[0], version.supports::<FileSourceId>());
                    assert_eq!($supports[1], version.supports::<GpsStandardTime>());
                    assert_eq!($supports[2], version.supports::<Waveforms>());
                    assert_eq!($supports[3], version.supports::<LargeFiles>());
                    assert_eq!($supports[4], version.supports::<Evlrs>());
                }

                #[test]
                fn point_formats() {
                    let version = Version::new($major, $minor);
                    for n in 0i8..11 {
                        let format = Format::new(n as u8).unwrap();
                        if n <= $max_point_format {
                            assert!(version.supports_point_format(format));
                        } else {
                            assert!(!version.supports_point_format(format));
                        }
                    }
                }
            }
        };
    }

    version!(las_1_0, 1, 0, [false; 5], 1);
    version!(las_1_1, 1, 1, [true, false, false, false, false], 1);
    version!(las_1_2, 1, 2, [true, true, false, false, false], 3);
    version!(las_1_3, 1, 3, [true, true, true, false, false], 5);
    version!(las_1_4, 1, 4, [true; 5], 10);
    version!(las_1_5, 1, 5, [false; 5], -1);
    version!(las_2_0, 2, 0, [false; 5], -1);
}
