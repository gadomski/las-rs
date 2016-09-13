/// An xyz collection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector<T> {
    /// X
    pub x: T,
    /// Y
    pub y: T,
    /// Z
    pub z: T,
}
