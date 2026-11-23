//! Where the E57 data comes from.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Error, Result};

/// A reader the converter can use from any thread.
pub(crate) trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

pub(crate) type BoxedReader = Box<dyn ReadSeek>;
pub(crate) type E57Reader = e57::E57Reader<BoxedReader>;

type OpenFn = dyn Fn() -> std::io::Result<BoxedReader> + Send + Sync;

/// The E57 data to convert.
///
/// Parallel conversion reads several point clouds at once, each with its own
/// reader, so an input is something that can be *opened* several times: a
/// path, bytes in memory, or a function returning a fresh reader.
///
/// Paths and byte buffers convert with `into()`:
///
/// ```no_run
/// use e57_to_las::{Converter, Input, output::FileOutput};
///
/// let from_path = Converter::new().convert("scan.e57", FileOutput::new("scan.las"))?;
///
/// let bytes: Vec<u8> = std::fs::read("scan.e57")?;
/// let from_memory = Converter::new().convert(bytes, FileOutput::new("scan.las"))?;
///
/// let custom = Input::from_fn(|| std::fs::File::open("scan.e57"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
pub struct Input {
    kind: Kind,
}

#[derive(Clone)]
enum Kind {
    Path(PathBuf),
    Bytes(Arc<[u8]>),
    Fn(Arc<OpenFn>),
}

impl Input {
    /// A file. Each reader gets its own buffered handle.
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Input {
            kind: Kind::Path(path.into()),
        }
    }

    /// An E57 file held in memory. Readers share the bytes without copying.
    pub fn bytes(bytes: impl Into<Arc<[u8]>>) -> Self {
        Input {
            kind: Kind::Bytes(bytes.into()),
        }
    }

    /// Any data source: `open` is called once per reader and must return an
    /// independent reader positioned at the start of the E57 data.
    ///
    /// Buffer the reader (with [`BufReader`]) if reads are expensive.
    pub fn from_fn<R, F>(open: F) -> Self
    where
        R: Read + Seek + Send + 'static,
        F: Fn() -> std::io::Result<R> + Send + Sync + 'static,
    {
        Input {
            kind: Kind::Fn(Arc::new(move || {
                open().map(|reader| Box::new(reader) as BoxedReader)
            })),
        }
    }

    /// The input path, if the input is a file.
    pub fn as_path(&self) -> Option<&Path> {
        match &self.kind {
            Kind::Path(path) => Some(path),
            _ => None,
        }
    }

    /// The file name without extension, used for the `{stem}` output placeholder.
    pub(crate) fn stem(&self) -> Option<String> {
        self.as_path()?
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
    }

    fn open_raw(&self) -> std::io::Result<BoxedReader> {
        Ok(match &self.kind {
            Kind::Path(path) => Box::new(BufReader::new(File::open(path)?)),
            Kind::Bytes(bytes) => Box::new(Cursor::new(Arc::clone(bytes))),
            Kind::Fn(open) => open()?,
        })
    }

    /// Opens a new E57 reader, which parses the file's XML section.
    pub(crate) fn open(&self) -> Result<E57Reader> {
        let reader = self.open_raw().map_err(|source| Error::Open {
            path: self.as_path().map(Path::to_path_buf),
            source,
        })?;
        e57::E57Reader::new(reader).map_err(|err| Error::InvalidE57(Box::new(err)))
    }
}

impl fmt::Debug for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::Path(path) => f.debug_tuple("Input::Path").field(path).finish(),
            Kind::Bytes(bytes) => write!(f, "Input::Bytes({} bytes)", bytes.len()),
            Kind::Fn(_) => f.write_str("Input::Fn(..)"),
        }
    }
}

macro_rules! from_path {
    ($($t:ty),*) => {$(
        impl From<$t> for Input {
            fn from(path: $t) -> Self {
                Input::path(path)
            }
        }
    )*};
}
from_path!(&Path, PathBuf, &PathBuf, &str, String, &String);

impl From<Vec<u8>> for Input {
    fn from(bytes: Vec<u8>) -> Self {
        Input::bytes(bytes)
    }
}

impl From<Arc<[u8]>> for Input {
    fn from(bytes: Arc<[u8]>) -> Self {
        Input::bytes(bytes)
    }
}

impl From<Box<[u8]>> for Input {
    fn from(bytes: Box<[u8]>) -> Self {
        Input::bytes(bytes)
    }
}
