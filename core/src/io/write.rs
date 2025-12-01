use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Cursor, Seek, Write},
};

use crate::Result;

/// Trait for types that can be consumed to create a LAS writer.
///
/// This trait is used for single-output conversion operations where a single
/// LAS output is created. Types like `PathBuf`, `String`, and owned collections
/// implement this trait.
///
/// See the [module documentation](crate::io) for implementation guidance and examples.
///
/// # Examples
///
/// ```no_run
/// use e57_to_las::{convert, ConvertOptions, io::WriterOnce};
/// use std::path::PathBuf;
///
/// // PathBuf implements WriterOnce
/// let output = PathBuf::from("output.las");
/// convert("scan.e57", output, &ConvertOptions::default())?;
/// ```
///
/// # See also
///
/// - [`WriterFactory`] for creating multiple outputs in split conversion
pub trait WriterOnce: Sized {
    /// The type of writer produced.
    type Writer: Write + Seek + Send + 'static;

    /// Convert this value into a writer.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be created (e.g., permission denied).
    fn try_into_writer(self) -> Result<Self::Writer>;
}

/// Trait for types that can create multiple LAS writers.
///
/// This trait is used for split conversion operations where each point cloud
/// is written to a separate LAS output. The implementation determines output
/// naming based on the point cloud context.
///
/// See the [module documentation](crate::io) for implementation guidance and examples.
///
/// # Examples
///
/// ```no_run
/// use e57_to_las::{convert_split, ConvertOptions, io::WriterFactory};
///
/// // &str implements WriterFactory
/// let output = "output.las";
/// let writers = convert_split("scan.e57", &output, &ConvertOptions::default())?;
/// println!("Created {} outputs", writers.len());
/// ```
///
/// # See also
///
/// - [`WriterOnce`] for single output
/// - [`WritePointCloudCtx`] for context provided to `create_writer`
pub trait WriterFactory {
    /// The type of writer produced.
    type Writer: Write + Seek + Send + 'static;

    /// Create a writer for a specific point cloud.
    ///
    /// The context provides information about which point cloud is being written,
    /// allowing the implementation to determine appropriate output names.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer cannot be created (e.g., permission denied,
    /// disk full).
    fn create_writer(&self, ctx: &WritePointCloudCtx) -> Result<Self::Writer>;
}

/// Context information for creating a point cloud writer.
///
/// This struct provides metadata about a point cloud to [`WriterFactory`]
/// implementations, allowing them to generate appropriate names.
///
/// # Naming
///
/// For path-based implementations:
/// - If `name` is `Some(n)`: `base_{n}.las`
/// - If `name` is `None`: `base_{idx}.las`
///
/// # Examples
///
/// Custom writer factory:
///
/// ```no_run
/// use e57_to_las::io::{WriterFactory, WritePointCloudCtx};
/// use std::io::{BufWriter, Write, Seek};
/// use std::fs::File;
///
/// struct MyFactory;
///
/// impl WriterFactory for MyFactory {
///     type Writer = BufWriter<File>;
///
///     fn create_writer(&self, ctx: &WritePointCloudCtx) -> e57_to_las::Result<Self::Writer> {
///         let filename = match ctx.name {
///             Some(name) => format!("cloud_{}.las", name),
///             None => format!("cloud_{}.las", ctx.idx),
///         };
///         Ok(BufWriter::new(File::create(filename)?))
///     }
/// }
/// ```
pub struct WritePointCloudCtx<'a> {
    /// Zero-based index of the point cloud within the E57 source.
    pub idx: usize,

    /// Optional name of the point cloud from E57 metadata.
    pub name: Option<&'a String>,
}

impl WriterOnce for File {
    type Writer = BufWriter<Self>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(self))
    }
}

impl<T> WriterOnce for BufWriter<T>
where
    T: Write + Seek + Send + 'static,
{
    type Writer = Self;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(self)
    }
}

impl WriterOnce for &'static mut [u8] {
    type Writer = BufWriter<Cursor<Self>>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(Cursor::new(self)))
    }
}

impl WriterOnce for Vec<u8> {
    type Writer = BufWriter<Cursor<Self>>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(Cursor::new(self)))
    }
}

impl WriterOnce for VecDeque<u8> {
    type Writer = BufWriter<Cursor<Vec<u8>>>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(Cursor::new(self.into())))
    }
}

impl<const N: usize> WriterOnce for [u8; N] {
    type Writer = BufWriter<Cursor<Self>>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(Cursor::new(self)))
    }
}

impl<const N: usize> WriterOnce for &'static mut [u8; N] {
    type Writer = BufWriter<Cursor<&'static mut [u8]>>;
    fn try_into_writer(self) -> Result<Self::Writer> {
        Ok(BufWriter::new(Cursor::new(self)))
    }
}

impl<F, W> WriterFactory for F
where
    F: Fn(&WritePointCloudCtx) -> Result<W>,
    W: Write + Seek + Send + 'static,
{
    type Writer = W;

    fn create_writer(&self, ctx: &WritePointCloudCtx) -> Result<Self::Writer> {
        self(ctx)
    }
}
