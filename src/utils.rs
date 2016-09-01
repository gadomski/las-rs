//! General-use structs and methods.

use std::ascii::AsciiExt;
use std::f64;
use std::str;

use {Error, Result};

/// x, y, and z values in one struct.
#[derive(Clone, Copy, Debug, PartialEq)]
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
}
