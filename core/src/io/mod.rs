//! I/O abstractions for reading E57 sources and writing LAS outputs.
//!
//! This module provides traits that abstract over different input and output types,
//! allowing the conversion functions to work with file system paths, in-memory buffers,
//! network streams, and custom I/O implementations.
//!
//! # Reading Sources
//!
//! E57 sources can be provided as:
//! - File paths: `&Path`, `PathBuf`, `&str`, `String`
//! - File handles: `File`, `BufReader<File>`
//! - Memory: `Vec<u8>`, `&[u8]`, arrays
//! - Custom implementations
//!
//! For single-use conversion ([`convert`](crate::convert)), use types implementing [`ReaderOnce`].
//! For parallel or split conversion, use types implementing [`ReaderFactory`] (typically references).
//!
//! # Writing Sinks
//!
//! LAS outputs can be written to:
//! - File paths: `&Path`, `PathBuf`, `&str`, `String`
//! - File handles: `File`, `BufWriter<File>`
//! - Memory: `Vec<u8>`, `&mut [u8]`, arrays
//! - Custom implementations
//!
//! For merged output ([`convert`](crate::convert)), use types implementing [`WriterOnce`].
//! For split output ([`convert_split`](crate::convert_split)), use types implementing [`WriterFactory`].
//!
//! # Implementing Custom I/O Types
//!
//! You can implement these traits for custom types to support specialized use cases beyond standard file I/O.
//!
//! ## Use Cases for Custom Implementations
//!
//! - **Network streams**: Read E57 from HTTP/cloud storage, write LAS to remote endpoints
//! - **In-memory processing**: Convert data without file system access
//! - **Compression**: Apply custom compression or encryption during conversion
//! - **Databases**: Read/write point clouds from database BLOBs
//! - **Testing**: Mock I/O for unit tests
//!
//! ## Performance
//!
//! **You should wrap your readers and writers in [`BufReader`] and [`BufWriter`] (or a custom buffering wrapper)** to avoid
//! performance degradation. The E57 and LAS formats perform many small read/write operations, and
//! unbuffered I/O can be 100-1000x slower.
//!
//! ## Example: Custom Network Source
//!
//! ```rust
//! use e57_to_las::io::{ReaderOnce, ReaderFactory};
//! use std::io::{BufReader, Cursor, Read, Seek};
//!
//! // Custom type that downloads E57 data from a URL
//! struct NetworkSource {
//!     url: String,
//! }
//!
//! impl NetworkSource {
//!     fn download(&self) -> e57_to_las::Result<Vec<u8>> {
//!         // Download implementation (simplified)
//!         # Ok(vec![])
//!         // let response = reqwest::blocking::get(&self.url)?;
//!         // Ok(response.bytes()?.to_vec())
//!     }
//! }
//!
//! impl ReaderOnce for NetworkSource {
//!     // Use BufReader for performance !
//!     type Reader = BufReader<Cursor<Vec<u8>>>;
//!
//!     fn try_into_reader(self) -> e57_to_las::Result<Self::Reader> {
//!         let data = self.download()?;
//!         // Wrap in BufReader to avoid performance issues
//!         Ok(BufReader::new(Cursor::new(data)))
//!     }
//! }
//!
//! // Now can use with conversion functions
//! use e57_to_las::{convert, ConvertOptions};
//! let source = NetworkSource { url: "https://example.com/scan.e57".to_string() };
//! convert(source, "output.las", &ConvertOptions::default())?;
//! ```
//!
//! ## Why `File` Cannot Implement `ReaderFactory` or `WriterFactory`
//!
//! You might notice that `File` implements `ReaderOnce` and `WriterOnce` but not `ReaderFactory` nor `WriterFactory`. This is a deliberate
//! safety decision:
//!
//! `ReaderFactory` and `WriterFactory` requires creating multiple independent readers/writers from the same source. While you could
//! theoretically clone a `File` handle, doing so creates multiple descriptors pointing to the same
//! underlying file with **shared seek positions**. This leads to data races and corrupted reads in parallel contexts.
//!
//! For parallel/factory operations, use path types (`&Path`, `&str`) instead, which safely
//! create independent file handles.
//!
//! ## Automatic Implementations for Path Types
//!
//! Most types implementing `AsRef<Path>` automatically get implementations of all four I/O traits
//! (`ReaderOnce`, `ReaderFactory`, `WriterOnce`, `WriterFactory`). These implementations:
//!
//! - Open files using `File::open()` or `File::create()`
//! - **Automatically wrap in `BufReader`/`BufWriter`** for optimal performance
//! - Handle naming for split conversions (appending point cloud names/indices)
//!
//! This covers the common use case with zero additional code.

mod read;
mod write;

pub use read::{ReaderFactory, ReaderOnce};
pub use write::{WritePointCloudCtx, WriterFactory, WriterOnce};

use crate::Result;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

macro_rules! impl_pathlike {
    ($t:ty) => {
        impl ReaderOnce for $t {
            type Reader = BufReader<File>;

            fn try_into_reader(self) -> Result<Self::Reader> {
                let path: &Path = self.as_ref();
                let file = File::open(path)?;
                Ok(BufReader::new(file))
            }
        }

        impl ReaderFactory for $t {
            type Reader = BufReader<File>;

            fn create_reader(&self) -> Result<Self::Reader> {
                let path: &Path = self.as_ref();
                let file = File::open(path)?;
                Ok(BufReader::new(file))
            }
        }

        impl WriterOnce for $t {
            type Writer = BufWriter<File>;

            fn try_into_writer(self) -> Result<Self::Writer> {
                let path: &Path = self.as_ref();
                let file = File::create(path)?;
                Ok(BufWriter::new(file))
            }
        }

        impl WriterFactory for $t {
            type Writer = BufWriter<File>;

            fn create_writer(&self, ctx: &WritePointCloudCtx) -> Result<Self::Writer> {
                let mut path = PathBuf::from(self);
                let cloud_id = match ctx.name {
                    Some(n) => n.to_owned(),
                    None => ctx.idx.to_string(),
                };

                match path.extension() {
                    Some(e) => {
                        let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        let filename = format!("{file_stem}_{cloud_id}.{}", e.to_string_lossy());
                        path.set_file_name(filename);
                    }
                    None => {
                        let filename = format!("{cloud_id}.las");
                        path = path.join(filename);
                    }
                };

                let file = File::create(path)?;

                Ok(BufWriter::new(file))
            }
        }
    };
}

impl_pathlike!(&Path);
impl_pathlike!(PathBuf);
impl_pathlike!(&PathBuf);
impl_pathlike!(&str);
impl_pathlike!(String);
impl_pathlike!(&String);
impl_pathlike!(std::ffi::OsString);
impl_pathlike!(&std::ffi::OsStr);
