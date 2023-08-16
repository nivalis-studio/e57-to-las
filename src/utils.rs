use anyhow::{Context, Result};
use nalgebra::{Point3, Quaternion, UnitQuaternion, Vector3};
use std::path::{Path, PathBuf};

pub fn to_u16(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 255.0) as u16
}

// maybe group those function in extend point struct at some point ?
// (2 extended points ?)
pub fn construct_las_path(input_path: &str, output_path: &str, index: usize) -> Result<PathBuf> {
    let output_dir_path = Path::new(output_path);

    let input_file_name = Path::new(input_path)
        .file_stem()
        .context("Couldn't read file stem.")?
        .to_str()
        .context("Invalid file stem encoding.")?;

    let output_sub_dir_path = output_dir_path.join(input_file_name);

    std::fs::create_dir_all(&output_sub_dir_path).context(format!(
        "Couldn't find or create output dir {}.",
        output_sub_dir_path
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    let las_path = output_sub_dir_path.join(format!("{}{}", index, ".las"));

    Ok(las_path)
}

pub fn get_rotations_and_translations(
    transform: &e57::Transform,
) -> (UnitQuaternion<f64>, Vector3<f64>) {
    let rotation = UnitQuaternion::from_quaternion(Quaternion::new(
        transform.rotation.w,
        transform.rotation.x,
        transform.rotation.y,
        transform.rotation.z,
    ));
    let translation = Vector3::new(
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
    );
    (rotation, translation)
}

pub fn extract_coordinates(p: &e57::Point) -> Option<Point3<f64>> {
    if p.cartesian_invalid == 2 && p.spherical_invalid == 2 {
        return None;
    }

    if p.cartesian_invalid == 0 {
        return Some(Point3::new(p.cartesian.x, p.cartesian.y, p.cartesian.z));
    }

    if p.spherical_invalid == 0 {
        let cos_ele = f64::cos(p.spherical.elevation);
        return Some(Point3::new(
            p.spherical.range * cos_ele * f64::cos(p.spherical.azimuth),
            p.spherical.range * cos_ele * f64::sin(p.spherical.azimuth),
            p.spherical.range * f64::sin(p.spherical.elevation),
        ));
    }

    return None;
}

pub fn get_intensity(intensity: f32, intensity_invalid: u8) -> u16 {
    // A value of zero means the intensity is valid, 1 means invalid.
    if intensity_invalid == 1 {
        return 0;
    }

    return (intensity.clamp(0.0, 1.0) * 65535.0) as u16;
}
