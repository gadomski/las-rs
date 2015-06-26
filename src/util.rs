//! Utility functions and structures.

/// Three values in x, y, z order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Triplet<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}
