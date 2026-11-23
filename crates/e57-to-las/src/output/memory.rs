use std::collections::BTreeMap;
use std::io::Cursor;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::{Output, Target};
use crate::GroupKey;

/// Keeps every output in memory, by group key.
///
/// ```no_run
/// use e57_to_las::{Converter, output::MemoryOutput};
///
/// let output = MemoryOutput::new();
/// Converter::new().convert("scan.e57", &output)?;
/// let files = output.into_files();
/// let las_bytes: &Vec<u8> = &files[&"merged".into()];
/// # Ok::<(), e57_to_las::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct MemoryOutput {
    files: Mutex<BTreeMap<GroupKey, Vec<u8>>>,
}

impl MemoryOutput {
    /// An empty output.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<GroupKey, Vec<u8>>> {
        // Only whole entries are inserted, so a poisoned map is still consistent.
        self.files.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Removes and returns the LAS data of one output.
    pub fn take(&self, key: &GroupKey) -> Option<Vec<u8>> {
        self.lock().remove(key)
    }

    /// The LAS data of every completed output, by group key.
    pub fn into_files(self) -> BTreeMap<GroupKey, Vec<u8>> {
        self.files
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

impl Output for MemoryOutput {
    type Writer = Cursor<Vec<u8>>;

    fn create(&self, _target: &Target<'_>) -> std::io::Result<Self::Writer> {
        Ok(Cursor::new(Vec::new()))
    }

    fn finish(&self, target: &Target<'_>, writer: Self::Writer) -> std::io::Result<()> {
        self.lock().insert(target.key.clone(), writer.into_inner());
        Ok(())
    }
}
