use {Point, Vector};
use std::f64;

/// Minimum and maximum bounds in three dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// The minimum values.
    pub min: Vector<f64>,
    /// The maximum values.
    pub max: Vector<f64>,
}

impl Bounds {
    /// Grows the bounds to encompass this point in xyz space.
    ///
    /// # Examples
    ///
    /// ```
    /// # use las::{Bounds, Point};
    /// let point = Point { x: 1., y: 2., z: 3., ..Default::default() };
    /// let mut bounds = Bounds { ..Default::default() };
    /// bounds.grow(&point);
    /// assert_eq!(1., bounds.min.x);
    /// ```
    pub fn grow(&mut self, point: &Point) {
        if point.x < self.min.x {
            self.min.x = point.x;
        }
        if point.y < self.min.y {
            self.min.y = point.y;
        }
        if point.z < self.min.z {
            self.min.z = point.z;
        }
        if point.x > self.max.x {
            self.max.x = point.x;
        }
        if point.y > self.max.y {
            self.max.y = point.y;
        }
        if point.z > self.max.z {
            self.max.z = point.z;
        }
    }
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            min: Vector {
                x: f64::INFINITY,
                y: f64::INFINITY,
                z: f64::INFINITY,
            },
            max: Vector {
                x: f64::NEG_INFINITY,
                y: f64::NEG_INFINITY,
                z: f64::NEG_INFINITY,
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use Point;

    #[test]
    fn grow() {
        let mut bounds = Bounds { ..Default::default() };
        bounds.grow(&Point {
            x: 1.,
            y: 2.,
            z: 3.,
            ..Default::default()
        });
        assert_eq!(1., bounds.min.x);
        assert_eq!(1., bounds.max.x);
        assert_eq!(2., bounds.min.y);
        assert_eq!(2., bounds.max.y);
        assert_eq!(3., bounds.min.z);
        assert_eq!(3., bounds.max.z);
        bounds.grow(&Point {
            x: 0.,
            y: 1.,
            z: 2.,
            ..Default::default()
        });
        assert_eq!(0., bounds.min.x);
        assert_eq!(1., bounds.max.x);
        assert_eq!(1., bounds.min.y);
        assert_eq!(2., bounds.max.y);
        assert_eq!(2., bounds.min.z);
        assert_eq!(3., bounds.max.z);
        bounds.grow(&Point {
            x: 2.,
            y: 3.,
            z: 4.,
            ..Default::default()
        });
        assert_eq!(0., bounds.min.x);
        assert_eq!(2., bounds.max.x);
        assert_eq!(1., bounds.min.y);
        assert_eq!(3., bounds.max.y);
        assert_eq!(2., bounds.min.z);
        assert_eq!(4., bounds.max.z);
    }
}
