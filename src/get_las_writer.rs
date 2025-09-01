use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::{Context, Result};
use las::Vector;
use uuid::Uuid;

use crate::LasVersion;

// Shared constants for scale calculations
const MIN_SCALE: f64 = 0.001;
const QUANTUM: f64 = 1e-4; // 0.0001 - the quantization step for scale values

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

pub(crate) fn get_las_writer(
    guid: Option<String>,
    output_path: PathBuf,
    max_cartesian: f64,
    has_color: bool,
    las_version: &LasVersion,
) -> Result<las::Writer<BufWriter<File>>> {
    let mut builder = las::Builder::from(las_version);
    builder.point_format.has_color = has_color;
    builder.generating_software = String::from("e57_to_las");
    let offset = 0.0;

    let scale = find_smallest_scale(max_cartesian);

    let transform = las::Transform { scale, offset };
    builder.transforms = Vector {
        x: transform,
        y: transform,
        z: transform,
    };
    builder.guid = Uuid::parse_str(&guid.unwrap_or(Uuid::new_v4().to_string()).replace("_", "-"))
        .unwrap_or(Uuid::new_v4());

    let header = builder.into_header().context("Error encountered: ")?;

    let writer = las::Writer::from_path(&output_path, header).context("Error encountered: ")?;

    Ok(writer)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!(scale + 1e-12 >= theoretical_min, 
                    "Scale {} should be >= theoretical min {} for value {}", 
                    scale, theoretical_min, value);
        }
    }
}
