use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) fn create_path(path: PathBuf) -> Result<PathBuf> {
    let parent = path.parent().ok_or(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Invalid path.",
    ))?;

    std::fs::create_dir_all(&parent).context(format!(
        "Couldn't find or create output dir {}.",
        parent
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    Ok(path)
}
