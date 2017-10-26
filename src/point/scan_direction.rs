/// The direction at which the scanner mirror was traveling at the time of pulse output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanDirection {
    /// The scan is moving from the right to the left.
    RightToLeft,
    /// The scan is moving from the left to the right.
    LeftToRight,
}

impl Default for ScanDirection {
    fn default() -> ScanDirection {
        ScanDirection::RightToLeft
    }
}
