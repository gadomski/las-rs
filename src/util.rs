//! Utility functions and structures.

/// Three values in x, y, z order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Triplet<T> {
    /// The x value.
    pub x: T,
    /// The y value.
    pub y: T,
    /// The z value.
    pub z: T,
}
