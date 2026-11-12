use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// `<output_dir>/las/<index>.las`
pub(crate) fn las_file_path(output_dir: &Path, index: usize) -> PathBuf {
    output_dir.join("las").join(format!("{index}.las"))
}

/// `<output_dir>/stations.json`
pub(crate) fn stations_file_path(output_dir: &Path) -> PathBuf {
    output_dir.join("stations.json")
}

pub(crate) fn ensure_parent_dir(path: PathBuf) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or(std::io::Error::other("Invalid path."))?;

    std::fs::create_dir_all(parent)
        .with_context(|| format!("Couldn't find or create output dir {}.", parent.display()))?;

    Ok(path)
}
