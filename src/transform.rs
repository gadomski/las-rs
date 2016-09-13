/// A scale and an offset that transforms xyz coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// The scale.
    pub scale: f64,
    /// The offset.
    pub offset: f64,
}

impl Transform {
    /// Applies this transform to an i32, returning a float.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Transform;
    /// let transform = Transform { scale: 2., offset: 1. };
    /// assert_eq!(3., transform.direct(1));
    /// ```
    pub fn direct(&self, n: i32) -> f64 {
        self.scale * n as f64 + self.offset
    }

    /// Applies the inverse transform, and rounds the result.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::Transform;
    /// let transform = Transform { scale: 2., offset: 1. };
    /// assert_eq!(1, transform.inverse(2.9));
    /// ```
    pub fn inverse(&self, n: f64) -> i32 {
        ((n - self.offset) / self.scale).round() as i32
    }
}

impl Default for Transform {
    fn default() -> Transform {
        Transform {
            scale: 1.,
            offset: 0.,
        }
    }
}
