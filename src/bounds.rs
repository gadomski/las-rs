use crate::{transform::RoundingMode, Point, Result, Transform, Vector};
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

    /// Transform the bounds to be compatible with the chosen transform. Otherwise, points may lay outside of the bounding box due to floating-point issues.
    ///
    /// # Example
    ///
    /// ```
    /// use las::{Bounds, Transform, Vector};
    ///
    /// let bounds = Bounds {
    ///     min: Vector {
    ///         x: -2.7868618965148926,
    ///         y: -0.9322229027748108,
    ///         z: -5.8063459396362305,
    ///     },
    ///     max: Vector {
    ///         x: 0.6091402173042297,
    ///         y: 1.5428568124771118,
    ///         z: -0.09441471844911575,
    ///     },
    /// };
    ///
    /// // Currently, the default scale is 0.001.
    /// let new_bounds = bounds.adapt(&Default::default()).unwrap();
    /// assert_eq!(new_bounds.max.z, -0.094);
    /// ```
    pub fn adapt(&self, transform: &Vector<Transform>) -> Result<Self> {
        use RoundingMode::*;

        fn inner_convert(value: f64, transform: &Transform, rounding: RoundingMode) -> Result<f64> {
            // During saving, an instance with +-inf is saved. We must consider for this corner case.
            if value.is_infinite() {
                return Ok(value);
            }
            Ok(transform.direct(transform.inverse_with_rounding_mode(value, rounding)?))
        }

        Ok(Self {
            min: Vector {
                x: inner_convert(self.min.x, &transform.x, Floor)?,
                y: inner_convert(self.min.y, &transform.y, Floor)?,
                z: inner_convert(self.min.z, &transform.z, Floor)?,
            },
            max: Vector {
                x: inner_convert(self.max.x, &transform.x, Ceil)?,
                y: inner_convert(self.max.y, &transform.y, Ceil)?,
                z: inner_convert(self.max.z, &transform.z, Ceil)?,
            },
        })
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
    use crate::Point;

    #[test]
    fn grow() {
        let mut bounds = Bounds {
            ..Default::default()
        };
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

    const EPSILON: f64 = 0.00000001;

    #[test]
    fn bounds_adapt_neg_tick_above() {
        assert_bounds_adapt_to_value(-1.0 + EPSILON);
    }

    #[test]
    fn bounds_adapt_pos_tick_above() {
        assert_bounds_adapt_to_value(1.0 + EPSILON);
    }

    #[test]
    fn bounds_adapt_neg_tick_below() {
        assert_bounds_adapt_to_value(-1.0 - EPSILON);
    }

    #[test]
    fn bounds_adapt_pos_tick_below() {
        assert_bounds_adapt_to_value(1.0 - EPSILON);
    }

    fn assert_bounds_adapt_to_value(n: f64) {
        let mut bounds = Bounds {
            ..Default::default()
        };

        let point = Point {
            x: n,
            y: n,
            z: n,
            ..Default::default()
        };

        bounds.grow(&point);

        let rounded_box = bounds.adapt(&Default::default()).unwrap();

        assert_point_inside_bounds(&point, &rounded_box);
    }

    fn assert_point_inside_bounds(p: &Point, b: &Bounds) {
        let x_inside = b.max.x >= p.x && b.min.x <= p.x;
        let y_inside = b.max.y >= p.y && b.min.y <= p.y;
        let z_inside = b.max.z >= p.z && b.min.z <= p.z;

        let inside = x_inside && y_inside && z_inside;

        if !inside {
            let bounds = b;
            let point = Vector::<f64> {
                x: p.x,
                y: p.y,
                z: p.z,
            };

            panic!(
                "Point was outside the calculated bounds:\n{:#?}\n\n",
                (point, bounds)
            );
        }
    }
}
