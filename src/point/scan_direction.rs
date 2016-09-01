const MASK: u8 = 0b01000000;

/// The direction at which the scanner mirror was travelling at the time of the output pulse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanDirection {
    /// Positive scan direction.
    Positive,
    /// Negative scan direction.
    Negative,
}

impl From<u8> for ScanDirection {
    fn from(n: u8) -> ScanDirection {
        match (n & MASK) >> 6 {
            0 => ScanDirection::Negative,
            1 => ScanDirection::Positive,
            _ => unreachable!(),
        }
    }
}

impl From<ScanDirection> for u8 {
    fn from(scan_direction: ScanDirection) -> u8 {
        match scan_direction {
            ScanDirection::Negative => 0,
            ScanDirection::Positive => MASK,
        }
    }
}

impl Default for ScanDirection {
    fn default() -> ScanDirection {
        ScanDirection::Positive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_direction_from_u8() {
        assert_eq!(ScanDirection::Positive, ScanDirection::from(0b01000000));
        assert_eq!(ScanDirection::Negative, ScanDirection::from(0));
        assert_eq!(0b01000000, u8::from(ScanDirection::Positive));
        assert_eq!(0, u8::from(ScanDirection::Negative));
    }
}
