//! Where LAS data goes.
//!
//! The converter splits the selected point clouds into groups (see
//! [`Grouping`](crate::Grouping)); each group becomes one LAS file, described
//! by a [`Target`]. An [`Output`] creates the writer for each target.
//!
//! Built in:
//! - [`FileOutput`]: files named from a path template, written atomically;
//! - [`MemoryOutput`]: LAS files kept in memory;
//! - [`from_fn`]: any `Write + Seek` returned by a closure.

mod file;
mod memory;

use std::fmt;
use std::io::{Seek, Write};
use std::marker::PhantomData;
use std::path::PathBuf;

pub use file::FileOutput;
pub use memory::MemoryOutput;

use crate::CloudInfo;

/// File format of an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum Format {
    /// Uncompressed LAS.
    #[default]
    Las,
    /// LAZ (compressed LAS). Needs the `laz` feature.
    Laz,
}

impl Format {
    /// `las` or `laz`.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Las => "las",
            Format::Laz => "laz",
        }
    }
}

/// Identifies one output: `"merged"` for [`Grouping::Merged`](crate::Grouping::Merged),
/// the point cloud index for [`Grouping::PerCloud`](crate::Grouping::PerCloud),
/// or the key returned by a custom grouping.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct GroupKey(String);

impl GroupKey {
    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GroupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for GroupKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for GroupKey {
    fn from(key: String) -> Self {
        GroupKey(key)
    }
}

impl From<&str> for GroupKey {
    fn from(key: &str) -> Self {
        GroupKey(key.to_owned())
    }
}

impl From<usize> for GroupKey {
    fn from(key: usize) -> Self {
        GroupKey(key.to_string())
    }
}

/// One output file to create: a group of point clouds.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub(crate) key: &'a GroupKey,
    pub(crate) clouds: &'a [CloudInfo],
    pub(crate) format: Format,
    pub(crate) input_stem: Option<&'a str>,
}

impl<'a> Target<'a> {
    /// The group key.
    pub fn key(&self) -> &'a GroupKey {
        self.key
    }

    /// The point clouds written to this output, in file order. Never empty.
    pub fn clouds(&self) -> &'a [CloudInfo] {
        self.clouds
    }

    /// The format of the output. When it is inferred from the location, it is
    /// [`Format::Las`] in calls to [`Output::location`].
    pub fn format(&self) -> Format {
        self.format
    }

    /// File name of the input without extension, when the input is a file.
    pub fn input_stem(&self) -> Option<&'a str> {
        self.input_stem
    }
}

/// Creates the writer of each output and settles it once complete.
///
/// For each [`Target`], the converter calls [`create`](Output::create) when
/// the first points are ready, then exactly one of
/// [`finish`](Output::finish), once the LAS data is complete, or
/// [`abort`](Output::abort), when the conversion fails or is cancelled.
/// Calls for different targets may come from different threads.
pub trait Output: Sync {
    /// The writer LAS data is written to.
    type Writer: Write + Seek + Send + Sync + 'static;

    /// Where the target is written, if it is a file. Used to infer the format
    /// from the extension, to detect two outputs written to the same place,
    /// and in the [`Summary`](crate::Summary).
    fn location(&self, target: &Target<'_>) -> Option<PathBuf> {
        let _ = target;
        None
    }

    /// Creates the writer for `target`.
    fn create(&self, target: &Target<'_>) -> std::io::Result<Self::Writer>;

    /// Called with the writer once its LAS data is complete and flushed by
    /// the LAS writer. The default drops it.
    fn finish(&self, target: &Target<'_>, writer: Self::Writer) -> std::io::Result<()> {
        let _ = target;
        drop(writer);
        Ok(())
    }

    /// Called when `target` will not be completed. The writer, if one was
    /// created, has already been dropped. The default does nothing.
    fn abort(&self, target: &Target<'_>) {
        let _ = target;
    }
}

impl<O: Output + ?Sized> Output for &O {
    type Writer = O::Writer;

    fn location(&self, target: &Target<'_>) -> Option<PathBuf> {
        (**self).location(target)
    }

    fn create(&self, target: &Target<'_>) -> std::io::Result<Self::Writer> {
        (**self).create(target)
    }

    fn finish(&self, target: &Target<'_>, writer: Self::Writer) -> std::io::Result<()> {
        (**self).finish(target, writer)
    }

    fn abort(&self, target: &Target<'_>) {
        (**self).abort(target);
    }
}

/// An [`Output`] that calls `create` for each target. See [`from_fn`].
pub struct FnOutput<F, W> {
    create: F,
    writer: PhantomData<fn() -> W>,
}

impl<F, W> fmt::Debug for FnOutput<F, W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FnOutput(..)")
    }
}

/// An output whose writers are created by `create`.
///
/// ```no_run
/// use std::fs::File;
/// use std::io::BufWriter;
/// use e57_to_las::{Converter, Grouping, output};
///
/// let out = output::from_fn(|target: &output::Target<'_>| {
///     let name = target.clouds()[0].name.clone().unwrap_or_else(|| target.key().to_string());
///     File::create(format!("/data/{name}.las")).map(BufWriter::new)
/// });
/// Converter::new().grouping(Grouping::PerCloud).convert("scan.e57", out)?;
/// # Ok::<(), e57_to_las::Error>(())
/// ```
pub fn from_fn<F, W>(create: F) -> FnOutput<F, W>
where
    F: Fn(&Target<'_>) -> std::io::Result<W> + Sync,
    W: Write + Seek + Send + Sync + 'static,
{
    FnOutput {
        create,
        writer: PhantomData,
    }
}

impl<F, W> Output for FnOutput<F, W>
where
    F: Fn(&Target<'_>) -> std::io::Result<W> + Sync,
    W: Write + Seek + Send + Sync + 'static,
{
    type Writer = W;

    fn create(&self, target: &Target<'_>) -> std::io::Result<W> {
        (self.create)(target)
    }
}
