use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
};

use crate::Result;

/// Trait for types that can be consumed to create an E57 reader.
///
/// This trait is used for single-pass conversion operations where the input source
/// is consumed. Types like `PathBuf`, `String`, and owned collections implement
/// this trait.
///
/// See the [module documentation](crate::io) for implementation guidance and examples.
///
/// # Examples
///
/// ```no_run
/// use e57_to_las::{convert, ConvertOptions, io::ReaderOnce};
/// use std::path::PathBuf;
///
/// // PathBuf implements ReaderOnce
/// let input = PathBuf::from("scan.e57");
/// convert(input, "output.las", &ConvertOptions::default())?;
/// ```
///
/// # See also
///
/// - [`ReaderFactory`] for reusable sources needed by parallel conversion
pub trait ReaderOnce: Sized {
    /// The type of reader produced.
    type Reader: Read + Seek + Send + 'static;

    /// Convert this value into a reader.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader cannot be created (e.g., file not found).
    fn try_into_reader(self) -> Result<Self::Reader>;
}

/// Trait for types that can create multiple E57 readers.
///
/// This trait is used for parallel conversion operations that need to create
/// multiple independent readers from the same source. Reference types like
/// `&Path`, `&str`, and closures implement this trait.
///
/// See the [module documentation](crate::io) for why `File` cannot implement this trait
/// and implementation guidance for custom types.
///
/// # Examples
///
/// ```no_run
/// # #[cfg(feature = "parallel")]
/// # fn example() -> e57_to_las::Result<()> {
/// use e57_to_las::{parallel, ConvertOptions, io::ReaderFactory};
///
/// // &str implements ReaderFactory
/// let input = "scan.e57";
/// parallel::convert(&input, "output.las", &ConvertOptions::default())?;
/// # Ok(())
/// # }
/// ```
///
/// # See also
///
/// - [`ReaderOnce`] for single-use sources
pub trait ReaderFactory {
    /// The type of reader produced.
    type Reader: Read + Seek + Send + 'static;

    /// Create a new reader.
    ///
    /// This method can be called multiple times to create independent readers
    /// from the same source.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader cannot be created (e.g., file not found).
    fn create_reader(&self) -> Result<Self::Reader>;
}

impl ReaderOnce for File {
    type Reader = BufReader<Self>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(self))
    }
}

impl<T> ReaderOnce for BufReader<T>
where
    T: Read + Seek + Send + 'static,
{
    type Reader = Self;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(self)
    }
}

impl ReaderOnce for &'static [u8] {
    type Reader = BufReader<Cursor<Self>>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(Cursor::new(self)))
    }
}

impl ReaderOnce for Vec<u8> {
    type Reader = BufReader<Cursor<Self>>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(Cursor::new(self)))
    }
}

impl ReaderOnce for VecDeque<u8> {
    type Reader = BufReader<Cursor<Vec<u8>>>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(Cursor::new(self.into())))
    }
}

impl<const N: usize> ReaderOnce for [u8; N] {
    type Reader = BufReader<Cursor<Self>>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(Cursor::new(self)))
    }
}

impl<const N: usize> ReaderOnce for &'static [u8; N] {
    type Reader = BufReader<Cursor<Self>>;
    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(Cursor::new(self)))
    }
}

impl<F, R> ReaderFactory for F
where
    F: Fn() -> Result<R>,
    R: Read + Seek + Send + 'static,
{
    type Reader = R;

    fn create_reader(&self) -> Result<Self::Reader> {
        self()
    }
}
