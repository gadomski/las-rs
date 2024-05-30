use std::fmt;

use crate::{point::Error, Result};

const TIME_FORMATS: &[u8] = &[1, 3, 4, 5, 6, 7, 8, 9, 10];
const COLOR_FORMATS: &[u8] = &[2, 3, 5, 7, 8, 10];
const WAVEFORM_FORMATS: &[u8] = &[4, 5, 9, 10];
const NIR_FORMATS: &[u8] = &[8, 10];
const IS_COMPRESSED_MASK: u8 = 0x80;

fn is_point_format_compressed(point_format_id: u8) -> bool {
    point_format_id & IS_COMPRESSED_MASK == IS_COMPRESSED_MASK
}

fn point_format_id_compressed_to_uncompressd(point_format_id: u8) -> u8 {
    point_format_id & 0x3f
}

fn point_format_id_uncompressed_to_compressed(point_format_id: u8) -> u8 {
    point_format_id | 0x80
}

/// Point formats are defined by the las spec.
///
/// As of las 1.4, there are eleven point formats (0-10). A new [Format] can be
/// created from its code and converted back into it:
///
/// ```
/// use las::point::Format;
///
/// let format_1 = Format::new(1).unwrap();
/// assert!(format_1.has_gps_time);
/// assert_eq!(1, format_1.to_u8().unwrap());
///
/// assert!(Format::new(11).is_err());
/// ```
///
/// Point formats can have extra bytes, which are user-defined attributes. Extra bytes were
/// introduced in las 1.4.
///
/// ```
/// use las::point::Format;
/// let mut format = Format::new(0).unwrap();
/// format.extra_bytes = 1;
/// assert_eq!(21, format.len());
/// ```
///
/// Certain combinations of attributes in a point format are illegal, e.g. gps time is required for
/// all formats >= 6:
///
/// ```
/// use las::point::Format;
/// let mut format = Format::new(6).unwrap();
/// format.has_gps_time = false;
/// assert!(format.to_u8().is_err());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Format {
    /// Does this point format include gps time?
    pub has_gps_time: bool,
    /// Does this point format include red, green, and blue colors?
    pub has_color: bool,
    /// Does this point format use two bytes for its flags and scaled scan angles?
    pub is_extended: bool,
    /// Does this point format have waveforms?
    pub has_waveform: bool,
    /// Does this point format have near infrared data?
    pub has_nir: bool,
    /// The number of extra bytes on each point.
    pub extra_bytes: u16,
    /// Is this point format compressed?
    pub is_compressed: bool,
}

#[allow(clippy::len_without_is_empty)]
impl Format {
    /// Creates a new point format from a u8.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Format;
    /// let format = Format::new(0).unwrap();
    /// assert!(!format.has_gps_time);
    /// assert!(!format.has_color);
    ///
    /// let format = Format::new(3).unwrap();
    /// assert!(format.has_gps_time);
    /// assert!(format.has_color);
    ///
    /// assert!(Format::new(11).is_err());
    /// ```
    pub fn new(n: u8) -> Result<Format> {
        let is_compressed = is_point_format_compressed(n);
        if n > 10 && !is_compressed {
            Err(Error::FormatNumber(n).into())
        } else {
            let n = point_format_id_compressed_to_uncompressd(n);
            Ok(Format {
                has_gps_time: TIME_FORMATS.contains(&n),
                has_color: COLOR_FORMATS.contains(&n),
                has_waveform: WAVEFORM_FORMATS.contains(&n),
                has_nir: NIR_FORMATS.contains(&n),
                is_extended: n >= 6,
                extra_bytes: 0,
                is_compressed,
            })
        }
    }

    /// Converts this point format into an extended format.
    ///
    /// "Extended" formats can contain more information per point, and must have gps time.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Format;
    /// let mut format = Format::default();
    /// assert!(!format.has_gps_time);
    /// assert!(!format.is_extended);
    /// format.extend();
    /// assert!(format.has_gps_time);
    /// assert!(format.is_extended);
    /// ```
    pub fn extend(&mut self) {
        self.has_gps_time = true;
        self.is_extended = true;
    }

    /// Returns this point format's length.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Format;
    /// let mut format = Format::new(0).unwrap();
    /// assert_eq!(20, format.len());
    /// format.has_gps_time = true;
    /// assert_eq!(28, format.len());
    /// ```
    pub fn len(&self) -> u16 {
        let mut len = if self.is_extended { 22 } else { 20 } + self.extra_bytes;
        if self.has_gps_time {
            len += 8;
        }
        if self.has_color {
            len += 6;
        }
        if self.has_nir {
            len += 2;
        }
        if self.has_waveform {
            len += 29;
        }
        len
    }

    /// Converts this point format to a u8.
    ///
    /// Can return an error if there is an invalid combination of attributes.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Format;
    /// let mut format = Format::default();
    /// assert_eq!(0, format.to_u8().unwrap());
    /// format.is_extended = true;
    /// assert!(format.to_u8().is_err());
    /// format.has_gps_time = true;
    /// assert_eq!(6, format.to_u8().unwrap());
    /// ```
    pub fn to_u8(&self) -> Result<u8> {
        if !cfg!(feature = "laz") && self.is_compressed {
            Err(Error::Format(*self).into())
        } else if self.is_extended {
            if self.has_gps_time {
                if self.has_color {
                    if self.has_nir {
                        if self.has_waveform {
                            Ok(10)
                        } else {
                            Ok(8)
                        }
                    } else if self.has_waveform {
                        Err(Error::Format(*self).into())
                    } else {
                        Ok(7)
                    }
                } else if self.has_nir {
                    Err(Error::Format(*self).into())
                } else if self.has_waveform {
                    Ok(9)
                } else {
                    Ok(6)
                }
            } else {
                Err(Error::Format(*self).into())
            }
        } else if self.has_nir {
            Err(Error::Format(*self).into())
        } else if self.has_waveform {
            if self.has_gps_time {
                if self.has_color {
                    Ok(5)
                } else {
                    Ok(4)
                }
            } else {
                Err(Error::Format(*self).into())
            }
        } else {
            let mut n = if self.has_gps_time { 1 } else { 0 };
            if self.has_color {
                n += 2;
            }
            Ok(n)
        }
    }

    /// When the data is compressed (LAZ) the point format id written in the
    /// header is slightly different to let readers know the data is compressed
    pub(crate) fn to_writable_u8(self) -> Result<u8> {
        self.to_u8().map(|id| {
            if self.is_compressed {
                point_format_id_uncompressed_to_compressed(id)
            } else {
                id
            }
        })
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(n) = self.to_u8() {
            write!(f, "point format {}", n)
        } else {
            write!(f, "point format that does not map onto a code: {:?}", self)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! format {
        ($name:ident, $n:expr, $expected:expr, $len:expr) => {
            mod $name {
                use crate::point::Format;

                #[test]
                fn new() {
                    assert_eq!($expected, Format::new($n).unwrap());
                }

                #[test]
                fn len() {
                    assert_eq!($len, Format::new($n).unwrap().len());
                }

                #[test]
                fn to_u8() {
                    assert_eq!($n, Format::new($n).unwrap().to_u8().unwrap());
                }
            }
        };
    }

    format!(format_0, 0, Format::default(), 20);
    format!(
        format_1,
        1,
        Format {
            has_gps_time: true,
            ..Default::default()
        },
        28
    );
    format!(
        format_2,
        2,
        Format {
            has_color: true,
            ..Default::default()
        },
        26
    );
    format!(
        format_3,
        3,
        Format {
            has_gps_time: true,
            has_color: true,
            ..Default::default()
        },
        34
    );
    format!(
        format_4,
        4,
        Format {
            has_gps_time: true,
            has_waveform: true,
            ..Default::default()
        },
        57
    );
    format!(
        format_5,
        5,
        Format {
            has_gps_time: true,
            has_color: true,
            has_waveform: true,
            ..Default::default()
        },
        63
    );
    format!(
        format_6,
        6,
        Format {
            has_gps_time: true,
            is_extended: true,
            ..Default::default()
        },
        30
    );
    format!(
        format_7,
        7,
        Format {
            has_gps_time: true,
            has_color: true,
            is_extended: true,
            ..Default::default()
        },
        36
    );
    format!(
        format_8,
        8,
        Format {
            has_gps_time: true,
            has_color: true,
            has_nir: true,
            is_extended: true,
            ..Default::default()
        },
        38
    );
    format!(
        format_9,
        9,
        Format {
            has_gps_time: true,
            has_waveform: true,
            is_extended: true,
            ..Default::default()
        },
        59
    );
    format!(
        format_10,
        10,
        Format {
            has_gps_time: true,
            has_color: true,
            has_nir: true,
            has_waveform: true,
            is_extended: true,
            ..Default::default()
        },
        67
    );

    #[test]
    fn waveform_without_gps_time() {
        let format = Format {
            has_waveform: true,
            ..Default::default()
        };
        assert!(format.to_u8().is_err());
    }

    #[test]
    fn extended_without_gps_time() {
        let format = Format {
            is_extended: true,
            ..Default::default()
        };
        assert!(format.to_u8().is_err());
    }

    #[test]
    fn nir_without_extended() {
        let format = Format {
            has_nir: true,
            ..Default::default()
        };
        assert!(format.to_u8().is_err());
    }

    #[test]
    fn nir_without_color() {
        let format = Format {
            is_extended: true,
            has_nir: true,
            ..Default::default()
        };
        assert!(format.to_u8().is_err());
    }

    #[test]
    fn extra_bytes() {
        let format = Format {
            extra_bytes: 1,
            ..Default::default()
        };
        assert_eq!(21, format.len());
    }

    #[test]
    fn is_compressed() {
        let format = Format {
            is_compressed: true,
            ..Default::default()
        };
        if cfg!(feature = "laz") {
            assert!(format.to_u8().is_ok());
        } else {
            assert!(format.to_u8().is_err());
        }
    }
}
