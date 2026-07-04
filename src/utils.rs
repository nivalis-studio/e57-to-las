use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) fn ensure_parent_dir(path: PathBuf) -> Result<PathBuf> {
    let parent = path.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Invalid path.",
    ))?;

    std::fs::create_dir_all(parent)
        .with_context(|| format!("Couldn't find or create output dir {}.", parent.display()))?;

    Ok(path)
}
