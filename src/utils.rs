use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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
