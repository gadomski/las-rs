//! Utility functions and structures.

/// Three f64 values in x, y, z order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Triplet {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
