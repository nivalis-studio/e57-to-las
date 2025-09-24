use crate::{Error, Result};
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

pub trait Reader: Read + Seek + Send + 'static {}

pub trait TryIntoReader {
    type Raw: Reader;

    fn try_into_reader(self) -> Result<Self::Raw>;
}

pub trait MakeReader: Clone + Send + 'static {
    type Raw: Reader;

    fn make_reader(&self) -> Result<Self::Raw>;
}

impl<T: Read + Seek + Send + 'static> Reader for T {}

impl TryIntoReader for String {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for &str {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for &Path {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for PathBuf {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for &String {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for &PathBuf {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for File {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(BufReader::new(self))
    }
}

impl<T: Reader> TryIntoReader for BufReader<T> {
    type Raw = Self;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(self)
    }
}

impl TryIntoReader for &'static [u8] {
    type Raw = Cursor<&'static [u8]>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl TryIntoReader for Vec<u8> {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl TryIntoReader for &'static Vec<u8> {
    type Raw = Cursor<&'static [u8]>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self.as_slice()))
    }
}

impl<const N: usize> TryIntoReader for [u8; N] {
    type Raw = Cursor<[u8; N]>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl<const N: usize> TryIntoReader for &'static [u8; N] {
    type Raw = Cursor<&'static [u8; N]>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl TryIntoReader for Box<[u8]> {
    type Raw = Cursor<Box<[u8]>>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl<T> TryIntoReader for Cursor<T>
where
    T: AsRef<[u8]> + Send + 'static,
    Cursor<T>: Read + Seek,
{
    type Raw = Self;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(self)
    }
}

impl<T: Reader> TryIntoReader for Box<T> {
    type Raw = T;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(*self)
    }
}

impl<T: TryIntoReader + Clone> TryIntoReader for std::rc::Rc<T> {
    type Raw = T::Raw;
    fn try_into_reader(self) -> Result<Self::Raw> {
        match std::rc::Rc::try_unwrap(self) {
            Ok(inner) => inner.try_into_reader(),
            Err(rc) => (*rc).clone().try_into_reader(),
        }
    }
}

impl<T: TryIntoReader + Clone> TryIntoReader for std::sync::Arc<T> {
    type Raw = T::Raw;
    fn try_into_reader(self) -> Result<Self::Raw> {
        match std::sync::Arc::try_unwrap(self) {
            Ok(inner) => inner.try_into_reader(),
            Err(arc) => (*arc).clone().try_into_reader(),
        }
    }
}

impl<T: TryIntoReader> TryIntoReader for Option<T> {
    type Raw = T::Raw;
    fn try_into_reader(self) -> Result<Self::Raw> {
        match self {
            Some(inner) => inner.try_into_reader(),
            None => Err(Error::Io(std::io::Error::other("No reader provided"))),
        }
    }
}

impl<T: TryIntoReader, E: Into<crate::Error>> TryIntoReader for std::result::Result<T, E> {
    type Raw = T::Raw;
    fn try_into_reader(self) -> Result<Self::Raw> {
        match self {
            Ok(inner) => inner.try_into_reader(),
            Err(e) => Err(e.into()),
        }
    }
}

impl TryIntoReader for std::collections::VecDeque<u8> {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self.into()))
    }
}

impl<'a> TryIntoReader for std::borrow::Cow<'a, str> {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self.as_ref())?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for std::ffi::OsString {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl TryIntoReader for &std::ffi::OsStr {
    type Raw = BufReader<File>;
    fn try_into_reader(self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl MakeReader for &'static str {
    type Raw = BufReader<File>;
    fn make_reader(&self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl MakeReader for String {
    type Raw = BufReader<File>;
    fn make_reader(&self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl MakeReader for PathBuf {
    type Raw = BufReader<File>;
    fn make_reader(&self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl MakeReader for &'static Path {
    type Raw = BufReader<File>;
    fn make_reader(&self) -> Result<Self::Raw> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl<R, F> MakeReader for F
where
    F: Fn() -> Result<R> + Clone + Send + 'static,
    R: TryIntoReader,
{
    type Raw = R::Raw;
    fn make_reader(&self) -> Result<Self::Raw> {
        self()?.try_into_reader()
    }
}
