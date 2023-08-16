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

pub fn get_transform(pointcloud: &e57::PointCloud) -> e57::Transform {
    pointcloud.transform.clone().unwrap_or(e57::Transform {
        rotation: e57::Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        translation: e57::Translation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    })
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
    if let Some(ref c) = p.cartesian {
        if let Some(invalid) = p.cartesian_invalid {
            if invalid != 0 {
                return None;
            }
        }
        Some(Point3::new(c.x, c.y, c.z))
    } else if let Some(ref s) = p.spherical {
        if let Some(invalid) = p.spherical_invalid {
            if invalid != 0 {
                return None;
            }
        }
        let cos_ele = f64::cos(s.elevation);
        Some(Point3::new(
            s.range * cos_ele * f64::cos(s.azimuth),
            s.range * cos_ele * f64::sin(s.azimuth),
            s.range * f64::sin(s.elevation),
        ))
    } else {
        None
    }
}

pub fn get_intensity(intensity: Option<f32>, intensity_invalid: Option<u8>) -> u16 {
    // A value of zero means the intensity is valid, 1 means invalid.
    if intensity_invalid.unwrap_or(0) == 1 {
        return 0;
    }

    return (intensity.unwrap_or(0.0).clamp(0.0, 1.0) * 65535.0) as u16;
}
