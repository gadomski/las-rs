/// The direction at which the scanner mirror was traveling at the time of pulse output.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScanDirection {
    /// The scan is moving from the right to the left.
    #[default]
    RightToLeft,
    /// The scan is moving from the left to the right.
    LeftToRight,
}
