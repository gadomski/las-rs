//! General-use structs and methods.

use std::ascii::AsciiExt;
use std::f64;
use std::iter;
use std::str;

use {Error, Result};

/// x, y, and z values in one struct.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Triple<T: Copy> {
    /// The x value of the triple.
    pub x: T,
    /// The y value of the triple.
    pub y: T,
    /// The z value of the triple.
    pub z: T,
}

impl<T: Copy> Triple<T> {
    /// Creates a new triple.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::utils::Triple;
    /// let triple = Triple::new(1., 2., 3.);
    /// ```
    pub fn new(x: T, y: T, z: T) -> Triple<T> {
        Triple { x: x, y: y, z: z }
    }
}

/// Three-dimensional bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds<T: Copy> {
    /// The smallest corner of the bounds.
    pub min: Triple<T>,
    /// The largest corner of the bounds.
    pub max: Triple<T>,
}

impl<T: Copy> Bounds<T> {
    /// Creates a new bounds from min and max values.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::utils::Bounds;
    /// let bounds = Bounds::new(1., 2., 3., 4., 5., 6.);
    /// ```
    pub fn new(minx: T, miny: T, minz: T, maxx: T, maxy: T, maxz: T) -> Bounds<T> {
        Bounds {
            min: Triple {
                x: minx,
                y: miny,
                z: minz,
            },
            max: Triple {
                x: maxx,
                y: maxy,
                z: maxz,
            },
        }
    }
}

impl Bounds<f64> {
    /// Grows the bounds to include the point.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::utils::{Triple, Bounds};
    /// let mut bounds = Bounds::new(1., 2., 3., 4., 5., 6.);
    /// let point = Triple { x: 10., y: 11., z: 12. };
    /// bounds.grow(point);
    /// assert_eq!(point, bounds.max);
    /// ```
    pub fn grow(&mut self, triple: Triple<f64>) {
        self.min.x = self.min.x.min(triple.x);
        self.min.y = self.min.y.min(triple.y);
        self.min.z = self.min.z.min(triple.z);
        self.max.x = self.max.x.max(triple.x);
        self.max.y = self.max.y.max(triple.y);
        self.max.z = self.max.z.max(triple.z);
    }
}

impl Default for Bounds<f64> {
    fn default() -> Bounds<f64> {
        Bounds {
            min: Triple {
                x: f64::INFINITY,
                y: f64::INFINITY,
                z: f64::INFINITY,
            },
            max: Triple {
                x: f64::NEG_INFINITY,
                y: f64::NEG_INFINITY,
                z: f64::NEG_INFINITY,
            },
        }
    }
}

/// Converts bytes into a string, following LAS rules.
///
/// LAS specifies that all string fields should be ASCII and nul filled, but not all LAS data in
/// the wild follows these rules (here's looking at you, Riegl). This trait has two methods, one
/// permissive (`to_las_str`) and one strict (`to_las_str_strict`). The first just does it's best
/// to produce some sort of `&str`, while the second checks the rules.
pub trait ToLasStr {
    /// Interprets the bytes as a `&str`, permissively.
    ///
    /// # Examples
    ///
    /// `[u8]` implements `ToLasStr`.
    ///
    /// ```
    /// use las::utils::ToLasStr;
    /// assert_eq!("LiDAR", [76, 105, 68, 65, 82, 0, 33].to_las_str().unwrap());
    /// ```
    fn to_las_str(&self) -> Result<&str>;

    /// Interprets the bytes as a `&str`, enforcing the LAS rules.
    ///
    /// # Examples
    ///
    /// `[u8]` implements `ToLasStr`.
    ///
    /// ```
    /// use las::utils::ToLasStr;
    /// assert!([76, 105, 68, 65, 82, 0, 33].to_las_str_strict().is_err());
    /// ```
    fn to_las_str_strict(&self) -> Result<&str>;
}

impl ToLasStr for [u8] {
    fn to_las_str(&self) -> Result<&str> {
        if let Some(idx) = self.iter().position(|&n| n == 0) {
                str::from_utf8(&self[0..idx])
            } else {
                str::from_utf8(&self)
            }
            .map_err(Error::from)
    }

    fn to_las_str_strict(&self) -> Result<&str> {
        let s = try!(if let Some(idx) = self.iter().position(|&n| n == 0) {
            if self[idx..].iter().all(|&n| n == 0) {
                str::from_utf8(&self[0..idx]).map_err(Error::from)
            } else {
                Err(Error::NotNulFilled(self.to_vec()))
            }
        } else {
            str::from_utf8(&self).map_err(Error::from)
        });
        if s.is_ascii() {
            Ok(s)
        } else {
            Err(Error::NotAscii(s.to_string()))
        }
    }
}

/// Converts a string into bytes, ensuring zero-fill.
pub trait FromLasStr {
    /// Modifies `self` to match the provided str.
    ///
    /// # Examples
    ///
    /// `&[u8]` implements `FromLasStr`:
    ///
    /// ```
    /// use las::utils::FromLasStr;
    /// let mut bytes = [1; 5];
    /// bytes.from_las_str("Beer").unwrap();
    /// assert_eq!([66, 101, 101, 114, 0], bytes);
    fn from_las_str(&mut self, s: &str) -> Result<()>;
}

impl<T: AsMut<[u8]>> FromLasStr for T {
    fn from_las_str(&mut self, s: &str) -> Result<()> {
        let count = self.as_mut().len();
        if s.len() > count {
            return Err(Error::TooLong(format!("{} is larger than {} bytes", s, count)));
        }
        for (a, b) in self.as_mut().iter_mut().zip(s.bytes().chain(iter::repeat(0))) {
            *a = b;
        }
        Ok(())
    }
}

/// A linear transformation.
///
/// If `y = ax + b`, `a` is the scale and `b` is the offset.
///
/// # Examples
///
/// Linear transforms can be created from `(f64, f64)`:
///
/// ```
/// # use las::utils::LinearTransform;
/// let transform = LinearTransform::from((1., 0.));
/// ```
///
/// The `scale * x + offset` version of the transformation can be computed with `direct`:
///
/// ```
/// # use las::utils::LinearTransform;
/// let transform = LinearTransform::from((2., 1.));
/// assert_eq!(7., transform.direct(3));
/// ```
///
/// The `inverse` computes `(x - offset) / scale`:
///
/// ```
/// # use las::utils::LinearTransform;
/// # let transform = LinearTransform::from((2., 1.));
/// assert_eq!(3, transform.inverse(7.));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct LinearTransform {
    /// The mutiplicative constant.
    pub scale: f64,
    /// The additive constant.
    pub offset: f64,
}

impl LinearTransform {
    /// Computes the forward (direct) transformation, `scale * x + offset`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::utils::LinearTransform;
    /// let transform = LinearTransform::from((2., 1.));
    /// assert_eq!(7., transform.direct(3));
    /// ```
    pub fn direct(&self, n: i32) -> f64 {
        self.scale * n as f64 + self.offset
    }

    /// Computes the backwards (inverse) transformation, `(x - offset) / scale`.
    ///
    /// The resulting float is rounded to the nearest integer.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::utils::LinearTransform;
    /// let transform = LinearTransform::from((2., 1.));
    /// assert_eq!(3, transform.inverse(7.));
    /// ```
    pub fn inverse(&self, n: f64) -> i32 {
        ((n - self.offset) / self.scale).round() as i32
    }
}

impl Default for LinearTransform {
    fn default() -> LinearTransform {
        LinearTransform {
            scale: 1.,
            offset: 0.,
        }
    }
}

impl From<(f64, f64)> for LinearTransform {
    fn from((scale, offset): (f64, f64)) -> LinearTransform {
        LinearTransform {
            scale: scale,
            offset: offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_new() {
        let bounds = Bounds::new(1., 2., 3., 4., 5., 6.);
        assert_eq!(1., bounds.min.x);
        assert_eq!(2., bounds.min.y);
        assert_eq!(3., bounds.min.z);
        assert_eq!(4., bounds.max.x);
        assert_eq!(5., bounds.max.y);
        assert_eq!(6., bounds.max.z);
    }

    #[test]
    fn bounds_grow() {
        let mut bounds: Bounds<f64> = Default::default();
        bounds.grow(Triple::new(1., 2., 3.));
        bounds.grow(Triple::new(-1., -2., -3.));
        assert_eq!(Bounds::new(-1., -2., -3., 1., 2., 3.), bounds);
    }

    #[test]
    fn to_las_str_empty() {
        let buf = [0; 0];
        assert_eq!("", buf.to_las_str().unwrap());
        assert_eq!("", buf.to_las_str_strict().unwrap());
    }

    #[test]
    fn to_las_str_one() {
        let buf = [76];
        assert_eq!("L", buf.to_las_str().unwrap());
        assert_eq!("L", buf.to_las_str_strict().unwrap());
    }

    #[test]
    fn to_las_str_not_filled() {
        let buf = [76, 0, 33];
        assert_eq!("L", buf.to_las_str().unwrap());
        assert!(buf.to_las_str_strict().is_err());
    }

    #[test]
    fn to_las_str_unicode() {
        let buf = [240, 159, 146, 150];
        assert_eq!("\u{1f496}", buf.to_las_str().unwrap());
        assert!(buf.to_las_str_strict().is_err());
    }

    #[test]
    fn linear_transformation_identity() {
        let transform = LinearTransform::from((1.0, 0.));
        assert_eq!(2., transform.direct(2));
        assert_eq!(2, transform.inverse(2.));
    }

    #[test]
    fn linear_transformation_changes() {
        let transform = LinearTransform::from((2.0, 1.));
        assert_eq!(7., transform.direct(3));
        assert_eq!(3, transform.inverse(7.));
    }

    #[test]
    fn linear_transformation_rounding() {
        let transform = LinearTransform::from((4., 0.));
        assert_eq!(1, transform.inverse(3.));
    }

    #[test]
    fn from_las_str_empty() {
        assert!([].from_las_str("").is_ok());
    }

    #[test]
    fn from_las_str_char() {
        let mut data = [0];
        data.from_las_str("B").unwrap();
        assert_eq!([66], data);
    }

    #[test]
    fn from_las_str_fill() {
        let mut data = [0, 1];
        data.from_las_str("B").unwrap();
        assert_eq!([66, 0], data);
    }

    #[test]
    fn from_las_str_too_many() {
        let mut data = [0];
        assert!(data.from_las_str("Be").is_err());
    }
}
