//! A LAS file that becomes visible only after it has been fully written.
//!
//! Points go to a sibling `*.las.tmp`. [`StagedLas::commit`] closes the writer
//! and renames that file onto the final path. Dropping a [`StagedLas`] without
//! committing closes the writer and deletes the temporary file, so a failed
//! conversion leaves no `.las` behind.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) type LasWriter = las::Writer<BufWriter<File>>;

pub(crate) struct StagedLas {
    final_path: PathBuf,
    temp_path: PathBuf,
    writer: Option<LasWriter>,
    committed: bool,
}

impl StagedLas {
    pub(crate) fn new(final_path: PathBuf) -> Self {
        let temp_path = final_path.with_extension("las.tmp");
        Self {
            final_path,
            temp_path,
            writer: None,
            committed: false,
        }
    }

    pub(crate) fn temp_path(&self) -> &Path {
        &self.temp_path
    }

    pub(crate) fn has_writer(&self) -> bool {
        self.writer.is_some()
    }

    pub(crate) fn writer_mut(&mut self) -> Result<&mut LasWriter> {
        self.writer.as_mut().context("LAS writer was not opened")
    }

    pub(crate) fn insert_writer(&mut self, writer: LasWriter) -> Result<()> {
        if self.writer.is_some() {
            return Err(anyhow::anyhow!("LAS writer is already open"));
        }
        self.writer = Some(writer);
        Ok(())
    }

    /// Closes the writer if one is open.
    ///
    /// A failed close forgets the writer. `las::Writer`'s `Drop` calls
    /// `close().expect(...)`, and `closed` is set only after a successful
    /// close, so a second failure (a full disk, for example) would panic and
    /// hide the error that abandoned this file.
    pub(crate) fn close_writer(&mut self) -> Result<()> {
        let Some(mut writer) = self.writer.take() else {
            return Ok(());
        };
        if let Err(err) = writer.close() {
            std::mem::forget(writer);
            return Err(err).context("Failed to close the writer: ");
        }
        Ok(())
    }

    /// Closes the writer and moves the temporary file onto the final path.
    pub(crate) fn commit(mut self) -> Result<()> {
        self.close_writer()?;
        std::fs::rename(&self.temp_path, &self.final_path).with_context(|| {
            format!(
                "Failed to publish LAS file to {}",
                self.final_path.display()
            )
        })?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for StagedLas {
    fn drop(&mut self) {
        // A failed close is ignored: the caller already has the error that
        // abandoned this file, and [`StagedLas::close_writer`] skips the panic
        // in `las::Writer`'s Drop.
        let _ = self.close_writer();
        if !self.committed {
            let _ = std::fs::remove_file(&self.temp_path);
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::LasVersion;
    use crate::get_las_writer::{PointBounds, get_las_writer};

    fn open_staged(dir: &Path, file_name: &str) -> (PathBuf, StagedLas) {
        let final_path = dir.join(file_name);
        let mut staged = StagedLas::new(final_path.clone());
        let version = LasVersion::new(1, 4).expect("valid version");
        let writer = get_las_writer(
            None,
            staged.temp_path().to_path_buf(),
            PointBounds::default(),
            false,
            &version,
        )
        .expect("writer");
        staged.insert_writer(writer).expect("insert writer");
        (final_path, staged)
    }

    #[test]
    fn commit_publishes_only_the_final_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (final_path, mut staged) = open_staged(dir.path(), "0.las");
        let temp_path = staged.temp_path().to_path_buf();
        staged
            .writer_mut()
            .expect("writer")
            .write_point(las::Point {
                x: 1.0,
                y: 2.0,
                z: 3.0,
                ..Default::default()
            })
            .expect("write point");

        staged.commit().expect("commit");

        assert!(final_path.is_file(), "final LAS file should be published");
        assert!(!temp_path.exists(), "temporary file should be renamed away");
        let reader = las::Reader::from_path(&final_path).expect("read las");
        assert_eq!(reader.header().number_of_points(), 1);
    }

    #[test]
    fn drop_removes_the_partial_file_and_keeps_an_existing_las() {
        let dir = tempfile::tempdir().expect("temp dir");
        let final_path = dir.path().join("0.las");
        std::fs::write(&final_path, b"previous").expect("seed previous output");
        let (_, mut staged) = open_staged(dir.path(), "0.las");
        let temp_path = staged.temp_path().to_path_buf();
        staged
            .writer_mut()
            .expect("writer")
            .write_point(las::Point::default())
            .expect("write point");
        assert!(temp_path.is_file(), "points should already be on disk");

        drop(staged);

        assert!(!temp_path.exists(), "partial file should be removed");
        assert_eq!(
            std::fs::read(&final_path).expect("read previous output"),
            b"previous"
        );
    }
}
