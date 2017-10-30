//! Structures that represent file-level features.

use Version;

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

macro_rules! feature {
    ($name:ident, $($versions:expr),+) => {
        #[derive(Clone, Copy, Debug)]
        #[allow(missing_docs)]
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
    }
}

feature!(FileSourceId, 1, 2, 3, 4);
feature!(GpsStandardTime, 2, 3, 4);
feature!(Waveforms, 3, 4);
feature!(SyntheticReturnNumbers, 3, 4);
feature!(LargeFiles, 4);
feature!(Evlrs, 4);
