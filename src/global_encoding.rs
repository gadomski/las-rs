//! Global properties about LAS data.

use std::fmt;

const MASK: u16 = 1;

/// Global properties about the file.
///
/// Introduced in LAS 1.2.
#[derive(Clone, Copy, Debug)]
pub struct GlobalEncoding {
    /// The gps time definition.
    pub gps_time: GpsTime,
}

impl From<u16> for GlobalEncoding {
    fn from(n: u16) -> GlobalEncoding {
        let gps_time = match n & MASK {
            0 => GpsTime::Week,
            1 => GpsTime::Standard,
            _ => unreachable!(),
        };
        GlobalEncoding { gps_time: gps_time }
    }
}

impl From<GlobalEncoding> for u16 {
    fn from(global_encoding: GlobalEncoding) -> u16 {
        match global_encoding.gps_time {
            GpsTime::Week => 0,
            GpsTime::Standard => 1,
        }
    }
}

impl Default for GlobalEncoding {
    fn default() -> GlobalEncoding {
        GlobalEncoding { gps_time: GpsTime::Week }
    }
}

/// The GPS time type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpsTime {
    /// GPS time in the point records in GPS week time.
    ///
    /// This is the same as all time records in LAS 1.0 and 1.1.
    Week,
    /// GPS time is standard GPS time (satellite GPS time) minus 1e9.
    Standard,
}

impl fmt::Display for GpsTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GpsTime::Week => write!(f, "GPS week time"),
            GpsTime::Standard => write!(f, "GPS standard time"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gps_time_type() {
        let global_encoding = GlobalEncoding::from(0);
        assert_eq!(GpsTime::Week, global_encoding.gps_time);
        let global_encoding = GlobalEncoding::from(1);
        assert_eq!(GpsTime::Standard, global_encoding.gps_time);
        assert_eq!(0u16, GlobalEncoding::from(0).into());
        assert_eq!(1u16, GlobalEncoding::from(1).into());
    }
}
