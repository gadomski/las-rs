//! Programmatically determine whether a las version supports a feature.
//!
//! Features are structures that implement the [Feature] trait. The most common
//! way to use features is via [Version::supports] or
//! [Version::verify_support_for]:
//!
//! ```
//! use las::feature::Waveforms;
//! use las::{Version, Error};
//!
//! let las_1_2 = Version::new(1, 2);
//! assert!(!las_1_2.supports::<Waveforms>());
//! assert!(las_1_2.verify_support_for::<Waveforms>().is_err());
//!
//! let las_1_4 = Version::new(1, 4);
//! assert!(las_1_4.supports::<Waveforms>());
//! assert!(las_1_4.verify_support_for::<Waveforms>().is_ok());
//! ```

use crate::Version;

const MAJOR: u8 = 1;

/// A trait implemented by each feature.
pub trait Feature {
    /// Is this feature supported by this version?
    ///
    /// # Examples
    ///
    /// ```
    /// use las::feature::{Waveforms, Feature};
    /// use las::Version;
    /// assert!(!Waveforms::is_supported_by(Version::new(1, 2)));
    /// assert!(Waveforms::is_supported_by(Version::new(1, 4)));
    /// ```
    fn is_supported_by(version: Version) -> bool;

    /// Returns the name of this feature.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::feature::{Waveforms, Feature};
    /// assert_eq!("Waveforms", Waveforms::name());
    /// ```
    fn name() -> &'static str;
}

macro_rules! features {
    (   $(
            $(#[$meta:meta])*
            $name:ident ($($versions:expr_2021),+);
        )+
    ) => {
        $(
            $(#[$meta])*
            #[derive(Clone, Copy, Debug)]
            pub struct $name {}

            impl Feature for $name {
                fn is_supported_by(version: Version) -> bool {
                    vec![$($versions),+]
                        .into_iter()
                        .map(|minor| Version::new(MAJOR, minor))
                        .any(|v| version == v)
                }

                fn name() -> &'static str {
                    stringify!($name)
                }
            }
        )+
    }
}

features! {
    /// Does the header allow a file source id, or is that field reserved?
    FileSourceId(1, 2, 3, 4);
    /// Is there a bit flag to set the type of time value in each point?
    GpsStandardTime(2, 3, 4);
    /// Does this file support waveforms?
    Waveforms(3, 4);
    /// Is there a bit flag to indicate synthetic return numbers?
    SyntheticReturnNumbers(3, 4);
    /// Does this file support 64-bit point counts?
    LargeFiles(4);
    /// Does this file support extended variable length records?
    Evlrs(4);
}
