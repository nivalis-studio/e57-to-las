use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::{Context, Result};
use las::Vector;
use uuid::Uuid;

use crate::LasVersion;

// Shared constants for scale calculations
const MIN_SCALE: f64 = 0.001;
const QUANTUM: f64 = 1e-4; // 0.0001 - the quantization step for scale values

/// Per-axis min/max bounds of the converted LAS points.
///
/// Used to derive per-axis offsets (bounds midpoint) and the smallest scale
/// that still fits the point extent in the i32 range of LAS coordinates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PointBounds {
    pub(crate) min: Vector<f64>,
    pub(crate) max: Vector<f64>,
}

impl Default for PointBounds {
    fn default() -> Self {
        Self {
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

impl PointBounds {
    /// Expands the bounds to include the given point.
    pub(crate) fn update(&mut self, point: &las::Point) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    /// Expands the bounds to include another set of bounds.
    pub(crate) fn merge(&mut self, other: &PointBounds) {
        self.min.x = self.min.x.min(other.min.x);
        self.min.y = self.min.y.min(other.min.y);
        self.min.z = self.min.z.min(other.min.z);
        self.max.x = self.max.x.max(other.max.x);
        self.max.y = self.max.y.max(other.max.y);
        self.max.z = self.max.z.max(other.max.z);
    }

    /// Returns true if no point was ever added to these bounds.
    fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }
}

fn find_smallest_scale(x: f64) -> f64 {
    // Early return for small values that work with the minimum scale
    if x.abs() <= f64::from(i32::MAX) * MIN_SCALE {
        return MIN_SCALE;
    }

    // Calculate the theoretical minimum scale needed
    // For a value x to fit in i32 range when divided by scale:
    // |x / scale| <= i32::MAX
    // scale >= |x| / i32::MAX
    let theoretical_min = x.abs() / f64::from(i32::MAX);

    // Quantize to QUANTUM steps (0.0001), matching the legacy increment
    let scale = ((theoretical_min / QUANTUM).ceil()) * QUANTUM;

    // Ensure we don't go below our minimum scale
    scale.max(MIN_SCALE)
}

/// Computes the LAS transform for one axis: the offset is the bounds midpoint
/// (rounded to whole meters for readable headers) and the scale is the
/// smallest quantized scale such that every value on the axis fits in i32
/// once the offset is subtracted.
fn axis_transform(min: f64, max: f64) -> las::Transform {
    let offset = ((min + max) / 2.0).round();

    // Largest distance from the (rounded) offset; the scale must fit this.
    let half_extent = (max - offset).abs().max((min - offset).abs());
    let scale = find_smallest_scale(half_extent);

    las::Transform { scale, offset }
}

pub(crate) fn get_las_writer(
    guid: Option<String>,
    output_path: PathBuf,
    bounds: PointBounds,
    has_color: bool,
    las_version: &LasVersion,
) -> Result<las::Writer<BufWriter<File>>> {
    let mut builder = las::Builder::from(las_version);
    builder.point_format.has_color = has_color;
    builder.generating_software = String::from("e57_to_las");

    builder.transforms = if bounds.is_empty() {
        // No valid points: fall back to the legacy default transform.
        let transform = las::Transform {
            scale: MIN_SCALE,
            offset: 0.0,
        };
        Vector {
            x: transform,
            y: transform,
            z: transform,
        }
    } else {
        Vector {
            x: axis_transform(bounds.min.x, bounds.max.x),
            y: axis_transform(bounds.min.y, bounds.max.y),
            z: axis_transform(bounds.min.z, bounds.max.z),
        }
    };
    builder.guid = match guid {
        Some(guid) => Uuid::parse_str(&guid.replace("_", "-")).unwrap_or_else(|_| {
            let fallback = Uuid::new_v4();
            eprintln!(
                "Warning: could not parse E57 guid {guid:?} as a UUID, using random guid {fallback} instead"
            );
            fallback
        }),
        None => Uuid::new_v4(),
    };

    let header = builder.into_header().context("Error encountered: ")?;

    let writer = las::Writer::from_path(&output_path, header).context("Error encountered: ")?;

    Ok(writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that every value in [min, max] fits in i32 with the transform.
    fn assert_fits(transform: &las::Transform, min: f64, max: f64) {
        for value in [min, max] {
            let scaled = ((value - transform.offset) / transform.scale).round();
            assert!(
                scaled >= f64::from(i32::MIN) && scaled <= f64::from(i32::MAX),
                "Value {value} does not fit with scale {} and offset {}",
                transform.scale,
                transform.offset
            );
        }
    }

    #[test]
    fn test_find_smallest_scale_small_values() {
        // Test values that should return the minimum scale
        assert_eq!(find_smallest_scale(0.0), MIN_SCALE);
        assert_eq!(find_smallest_scale(1.0), MIN_SCALE);
        assert_eq!(find_smallest_scale(1000.0), MIN_SCALE);
        assert_eq!(find_smallest_scale(-1000.0), MIN_SCALE);

        // Maximum value that still works with minimum scale
        let max_with_min_scale = f64::from(i32::MAX) * MIN_SCALE;
        assert_eq!(find_smallest_scale(max_with_min_scale), MIN_SCALE);
    }

    #[test]
    fn test_find_smallest_scale_large_values() {
        // Test large values that need larger scales
        let large_value = 1e10;
        let scale = find_smallest_scale(large_value);

        // Verify the scale works (value fits in i32 range when divided)
        let scaled = (large_value / scale).round();
        assert!(scaled >= f64::from(i32::MIN));
        assert!(scaled <= f64::from(i32::MAX));

        // Check scale is properly rounded to 0.0001 precision
        let rounded_scale = (scale * 10000.0).round() / 10000.0;
        assert!((scale - rounded_scale).abs() < 1e-10);
    }

    #[test]
    fn test_find_smallest_scale_boundary() {
        // Test value just above the threshold
        let just_above = f64::from(i32::MAX) * MIN_SCALE + 1.0;
        let scale = find_smallest_scale(just_above);
        assert!(scale > MIN_SCALE);

        // Verify it still works
        let scaled = (just_above / scale).round();
        assert!(scaled <= f64::from(i32::MAX));

        // Half-step close to the threshold
        let half_step = f64::from(i32::MAX) * MIN_SCALE + (MIN_SCALE / 2.0);
        let s2 = find_smallest_scale(half_step);
        let scaled2 = (half_step / s2).round();
        assert!(scaled2 <= f64::from(i32::MAX));
    }

    #[test]
    fn test_find_smallest_scale_negative() {
        // Test negative values
        let negative_large = -1e10;
        let scale = find_smallest_scale(negative_large);

        // Verify the scale works for negative values
        let scaled = (negative_large / scale).round();
        assert!(scaled >= f64::from(i32::MIN));
        assert!(scaled <= f64::from(i32::MAX));
    }

    #[test]
    fn test_scale_precision() {
        // Test that scales are rounded to QUANTUM precision, including negatives
        let test_values = [2.15e9, 3.7e9, 5.5e9, 1e11, -2.15e9, -1e11];

        for value in test_values {
            let scale = find_smallest_scale(value);
            // Check that scale has at most 4 decimal places
            let multiplied = scale * 10000.0;
            assert!((multiplied - multiplied.round()).abs() < 1e-10);

            // And that it meets the theoretical lower bound
            let theoretical_min = value.abs() / f64::from(i32::MAX);
            assert!(
                scale + 1e-12 >= theoretical_min,
                "Scale {scale} should be >= theoretical min {theoretical_min} for value {value}",
            );
        }
    }

    #[test]
    fn test_axis_transform_small_extent() {
        // Small extents around the origin keep the minimum scale
        let transform = axis_transform(-10.0, 10.0);
        assert_eq!(transform.scale, MIN_SCALE);
        assert_eq!(transform.offset, 0.0);
        assert_fits(&transform, -10.0, 10.0);
    }

    #[test]
    fn test_axis_transform_georeferenced_utm() {
        // A georeferenced (UTM-like) cloud: large coordinates, small extent.
        // With the legacy offset = 0 this required scale ~ 0.0024 (2.4 mm
        // quantization). With the midpoint offset the minimum scale fits.
        let (min, max) = (5_000_000.0, 5_000_100.0);
        let transform = axis_transform(min, max);
        assert_eq!(transform.scale, MIN_SCALE);
        assert_eq!(transform.offset, 5_000_050.0);
        assert_fits(&transform, min, max);
    }

    #[test]
    fn test_axis_transform_large_extent() {
        // Extents too large for the minimum scale still fit in i32
        let (min, max) = (-1e10, 1e10);
        let transform = axis_transform(min, max);
        assert!(transform.scale > MIN_SCALE);
        assert_fits(&transform, min, max);

        // Scale stays quantized to QUANTUM steps
        let multiplied = transform.scale * 10000.0;
        assert!((multiplied - multiplied.round()).abs() < 1e-10);
    }

    #[test]
    fn test_axis_transform_asymmetric_extent() {
        // Offset rounding must not break the fit for asymmetric bounds
        let (min, max) = (-3.5e6, 9.7e6 + 0.4321);
        let transform = axis_transform(min, max);
        assert_fits(&transform, min, max);
    }

    #[test]
    fn test_point_bounds_update_and_merge() {
        let mut bounds = PointBounds::default();
        assert!(bounds.is_empty());

        bounds.update(&las::Point {
            x: 1.0,
            y: -2.0,
            z: 3.0,
            ..Default::default()
        });
        bounds.update(&las::Point {
            x: -4.0,
            y: 5.0,
            z: 0.5,
            ..Default::default()
        });
        assert!(!bounds.is_empty());
        assert_eq!(bounds.min.x, -4.0);
        assert_eq!(bounds.max.x, 1.0);
        assert_eq!(bounds.min.y, -2.0);
        assert_eq!(bounds.max.y, 5.0);
        assert_eq!(bounds.min.z, 0.5);
        assert_eq!(bounds.max.z, 3.0);

        let mut other = PointBounds::default();
        other.update(&las::Point {
            x: 10.0,
            y: -20.0,
            z: 30.0,
            ..Default::default()
        });
        bounds.merge(&other);
        assert_eq!(bounds.max.x, 10.0);
        assert_eq!(bounds.min.y, -20.0);
        assert_eq!(bounds.max.z, 30.0);

        // Merging an empty bounds is a no-op
        bounds.merge(&PointBounds::default());
        assert_eq!(bounds.min.x, -4.0);
        assert_eq!(bounds.max.x, 10.0);
    }
}
