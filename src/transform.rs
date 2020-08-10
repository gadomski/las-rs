use std::fmt;
use Result;

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
    /// use las::Transform;
    /// let transform = Transform { scale: 2., offset: 1. };
    /// assert_eq!(3., transform.direct(1));
    /// ```
    pub fn direct(&self, n: i32) -> f64 {
        self.scale * f64::from(n) + self.offset
    }

    /// Applies the inverse transform, and rounds the result.
    ///
    /// Returns an error if the resultant value can't be represented as an i32.
    ///
    /// # Examples
    ///
    /// ```
    /// use las::Transform;
    /// let transform = Transform { scale: 2., offset: 1. };
    /// assert_eq!(1, transform.inverse(2.9).unwrap());
    /// ```
    pub fn inverse(&self, n: f64) -> Result<i32> {
        use std::i32;
        use Error;

        let n = ((n - self.offset) / self.scale).round();
        if n > f64::from(i32::MAX) || n < f64::from(i32::MIN) {
            Err(Error::InverseTransform {
                n,
                transform: *self,
            })
        } else {
            Ok(n as i32)
        }
    }
}

impl Default for Transform {
    fn default() -> Transform {
        Transform {
            scale: 0.001,
            offset: 0.,
        }
    }
}

impl fmt::Display for Transform {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "`{} * x + {}`", self.scale, self.offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::i32;

    #[test]
    fn too_large() {
        let transform = Transform::default();
        let n = i32::MAX as f64 * transform.scale + 1.;
        assert!(transform.inverse(n).is_err());
    }

    #[test]
    fn too_small() {
        let transform = Transform::default();
        let n = i32::MIN as f64 * transform.scale - 1.;
        assert!(transform.inverse(n).is_err());
    }
}
