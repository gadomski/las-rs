/// The direction at which the scanner mirror was traveling at the time of pulse output.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanDirection {
    /// The scan is moving from the right to the left.
    Negative,
    /// The scan is moving from the left to the right.
    Positive,
}

impl Default for ScanDirection {
    fn default() -> ScanDirection {
        ScanDirection::Negative
    }
}
