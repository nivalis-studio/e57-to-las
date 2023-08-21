use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn to_u16(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 255.0) as u16
}

// maybe group those function in extend point struct at some point ?
// (2 extended points ?)
pub fn construct_las_path(output_path: &Path, index: usize) -> Result<PathBuf> {
    let output_sub_dir_path = output_path.join("las");

    std::fs::create_dir_all(&output_sub_dir_path).context(format!(
        "Couldn't find or create output dir {}.",
        output_sub_dir_path
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    let las_path = output_sub_dir_path.join(format!("{}{}", index, ".las"));

    Ok(las_path)
}

pub fn get_intensity(intensity: f32, intensity_invalid: u8) -> u16 {
    // A value of zero means the intensity is valid, 1 means invalid.
    if intensity_invalid == 1 {
        return 0;
    }

    return (intensity.clamp(0.0, 1.0) * 65535.0) as u16;
}