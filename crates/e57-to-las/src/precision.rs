//! LAS scale and offset.
//!
//! LAS stores each coordinate as an `i32` times a scale, plus an offset. With a
//! 1 mm scale, points must lie within about 2,147 km of the offset.

use crate::cloud::Bounds;
use crate::{Error, Result};

/// How the LAS scale and offset are chosen.
///
/// ```
/// use e57_to_las::{Converter, Precision};
///
/// let converter = Converter::new().precision(Precision::Scale(0.0001)); // 0.1 mm
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Precision {
    /// 1 mm scale; the offset is chosen automatically. The default.
    #[default]
    Auto,
    /// The given scale (in coordinate units, usually meters); the offset is
    /// chosen automatically.
    Scale(f64),
    /// Exactly this scale and offset per axis. A point that does not fit fails
    /// the conversion with [`Error::PointOutOfRange`].
    Fixed {
        /// Scale `[x, y, z]`.
        scale: [f64; 3],
        /// Offset `[x, y, z]`.
        offset: [f64; 3],
    },
}

/// The default scale: 1 mm.
pub(crate) const DEFAULT_SCALE: f64 = 0.001;

impl Precision {
    /// The scale to aim for when it is chosen automatically.
    pub(crate) fn target_scale(&self) -> Option<f64> {
        match self {
            Precision::Auto => Some(DEFAULT_SCALE),
            Precision::Scale(scale) => Some(*scale),
            Precision::Fixed { .. } => None,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        let valid_scale = |s: f64| s.is_finite() && s > 0.0;
        let ok = match self {
            Precision::Auto => true,
            Precision::Scale(scale) => valid_scale(*scale),
            Precision::Fixed { scale, offset } => {
                scale.iter().all(|s| valid_scale(*s)) && offset.iter().all(|o| o.is_finite())
            }
        };
        if ok {
            Ok(())
        } else {
            Err(Error::InvalidOption(format!(
                "precision {self:?}: scales must be finite and positive, offsets finite"
            )))
        }
    }
}

/// Scale and offset of the three axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Transforms {
    pub(crate) scale: [f64; 3],
    pub(crate) offset: [f64; 3],
}

impl Transforms {
    /// Offset at `point` rounded to whole units, with `scale` on every axis.
    pub(crate) fn around(point: [f64; 3], scale: f64) -> Self {
        Transforms {
            scale: [scale; 3],
            offset: point.map(f64::round),
        }
    }

    /// Offset at the middle of `bounds` (rounded to whole units) and, per axis,
    /// the smallest scale no finer than `scale` that fits every value in `bounds`.
    pub(crate) fn fitting(bounds: &Bounds, scale: f64) -> Self {
        if bounds.is_empty() {
            return Transforms::around([0.0; 3], scale);
        }
        let mut out = Transforms::around([0.0; 3], scale);
        for axis in 0..3 {
            let (min, max) = (bounds.min[axis], bounds.max[axis]);
            let offset = ((min + max) / 2.0).round();
            let half_extent = (max - offset).abs().max((min - offset).abs());
            out.offset[axis] = offset;
            out.scale[axis] = smallest_scale(half_extent, scale);
        }
        out
    }

    /// The raw `i32` LAS coordinates of `point`, or `None` if one does not fit.
    #[inline]
    pub(crate) fn quantize(&self, point: [f64; 3]) -> Option<[i32; 3]> {
        let mut out = [0; 3];
        for axis in 0..3 {
            // Same rounding as `las::Transform::inverse`, so the header bounds
            // computed by `las` from these integers match.
            let n = ((point[axis] - self.offset[axis]) / self.scale[axis]).round();
            if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&n) {
                return None;
            }
            out[axis] = n as i32;
        }
        Some(out)
    }

    pub(crate) fn to_las(self) -> las::Vector<las::Transform> {
        let axis = |i: usize| las::Transform {
            scale: self.scale[i],
            offset: self.offset[i],
        };
        las::Vector {
            x: axis(0),
            y: axis(1),
            z: axis(2),
        }
    }
}

/// The smallest scale, no finer than `min_scale` and on a grid of a tenth of
/// it, such that `half_extent / scale` fits in an `i32`.
fn smallest_scale(half_extent: f64, min_scale: f64) -> f64 {
    let limit = f64::from(i32::MAX);
    if half_extent.abs() <= limit * min_scale {
        return min_scale;
    }
    let quantum = min_scale / 10.0;
    let scale = (half_extent.abs() / limit / quantum).ceil() * quantum;
    // Guard against the rounding of the division landing just below the limit.
    if half_extent.abs() / scale > limit {
        scale + quantum
    } else {
        scale
    }
    .max(min_scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fits(t: &Transforms, bounds: &Bounds) -> bool {
        t.quantize(bounds.min).is_some() && t.quantize(bounds.max).is_some()
    }

    #[test]
    fn small_extent_keeps_millimeters() {
        let bounds = Bounds {
            min: [-10.0, -10.0, -10.0],
            max: [10.0, 10.0, 10.0],
        };
        let t = Transforms::fitting(&bounds, DEFAULT_SCALE);
        assert_eq!(t.scale, [DEFAULT_SCALE; 3]);
        assert_eq!(t.offset, [0.0; 3]);
        assert!(fits(&t, &bounds));
    }

    #[test]
    fn georeferenced_cloud_keeps_millimeters() {
        // Large coordinates, small extent: the offset absorbs the coordinates.
        let bounds = Bounds {
            min: [650_000.0, 6_860_000.0, 30.0],
            max: [650_100.0, 6_860_100.0, 80.0],
        };
        let t = Transforms::fitting(&bounds, DEFAULT_SCALE);
        assert_eq!(t.scale, [DEFAULT_SCALE; 3]);
        assert_eq!(t.offset, [650_050.0, 6_860_050.0, 55.0]);
        assert!(fits(&t, &bounds));
    }

    #[test]
    fn huge_extent_gets_a_coarser_scale_that_fits() {
        let bounds = Bounds {
            min: [-1e10, -3.5e6, 0.0],
            max: [1e10, 9.7e6 + 0.4321, 1.0],
        };
        let t = Transforms::fitting(&bounds, DEFAULT_SCALE);
        assert!(t.scale[0] > DEFAULT_SCALE);
        assert!(t.scale[1] > DEFAULT_SCALE);
        assert_eq!(t.scale[2], DEFAULT_SCALE);
        assert!(fits(&t, &bounds));
    }

    #[test]
    fn smallest_scale_is_quantized_and_sufficient() {
        for value in [2.15e9, 3.7e9, 5.5e9, 1e11, -2.15e9, -1e11] {
            let scale = smallest_scale(value, DEFAULT_SCALE);
            let steps = scale / (DEFAULT_SCALE / 10.0);
            assert!(
                (steps - steps.round()).abs() < 1e-6,
                "{scale} not on the grid"
            );
            assert!(value.abs() / scale <= f64::from(i32::MAX));
        }
    }

    #[test]
    fn quantize_rejects_far_points() {
        let t = Transforms::around([0.0; 3], DEFAULT_SCALE);
        assert_eq!(t.quantize([1.0, -2.0, 0.0005]), Some([1000, -2000, 1]));
        assert_eq!(t.quantize([3e6, 0.0, 0.0]), None);
    }

    #[test]
    fn validation() {
        assert!(Precision::Scale(0.0).validate().is_err());
        assert!(Precision::Scale(f64::NAN).validate().is_err());
        assert!(Precision::Scale(0.01).validate().is_ok());
        assert!(
            Precision::Fixed {
                scale: [0.001; 3],
                offset: [0.0, f64::INFINITY, 0.0]
            }
            .validate()
            .is_err()
        );
    }
}
