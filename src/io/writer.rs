use crate::{Error, Result};
use std::{
    fs::{File, create_dir_all},
    io::{BufWriter, Cursor, Seek, Write},
    path::{Path, PathBuf},
};

pub trait Writer: Write + Seek + Send + 'static {}

pub trait TryIntoWriter {
    type Raw: Writer;

    fn try_into_writer(self) -> Result<Self::Raw>;
}

pub trait MakeWriter: Clone + Send + 'static {
    type Raw: Writer;

    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw>;
}

impl<T: Write + Seek + Send + 'static> Writer for T {}

impl TryIntoWriter for String {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for &str {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for &Path {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for PathBuf {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for &String {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for &PathBuf {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for File {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(BufWriter::new(self))
    }
}

impl<T: Writer> TryIntoWriter for BufWriter<T> {
    type Raw = Self;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(self)
    }
}

impl TryIntoWriter for Cursor<Vec<u8>> {
    type Raw = Self;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(self)
    }
}

impl TryIntoWriter for Vec<u8> {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self))
    }
}

impl TryIntoWriter for usize {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(Cursor::new(Vec::with_capacity(self)))
    }
}

impl TryIntoWriter for () {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(Cursor::new(Vec::new()))
    }
}

impl<T: Writer> TryIntoWriter for Box<T> {
    type Raw = T;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(*self)
    }
}

impl<T: TryIntoWriter + Clone> TryIntoWriter for std::rc::Rc<T> {
    type Raw = T::Raw;
    fn try_into_writer(self) -> Result<Self::Raw> {
        match std::rc::Rc::try_unwrap(self) {
            Ok(inner) => inner.try_into_writer(),
            Err(rc) => (*rc).clone().try_into_writer(),
        }
    }
}

impl<T: TryIntoWriter + Clone> TryIntoWriter for std::sync::Arc<T> {
    type Raw = T::Raw;
    fn try_into_writer(self) -> Result<Self::Raw> {
        match std::sync::Arc::try_unwrap(self) {
            Ok(inner) => inner.try_into_writer(),
            Err(arc) => (*arc).clone().try_into_writer(),
        }
    }
}

impl<T: TryIntoWriter> TryIntoWriter for Option<T> {
    type Raw = T::Raw;
    fn try_into_writer(self) -> Result<Self::Raw> {
        match self {
            Some(inner) => inner.try_into_writer(),
            None => Err(Error::Io(std::io::Error::other("No writer provided"))),
        }
    }
}

impl<T: TryIntoWriter, E: Into<crate::Error>> TryIntoWriter for std::result::Result<T, E> {
    type Raw = T::Raw;
    fn try_into_writer(self) -> Result<Self::Raw> {
        match self {
            Ok(inner) => inner.try_into_writer(),
            Err(e) => Err(e.into()),
        }
    }
}

impl TryIntoWriter for Box<[u8]> {
    type Raw = Cursor<Vec<u8>>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        Ok(Cursor::new(self.into_vec()))
    }
}

impl<'a> TryIntoWriter for std::borrow::Cow<'a, str> {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self.as_ref())?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for std::ffi::OsString {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl TryIntoWriter for &std::ffi::OsStr {
    type Raw = BufWriter<File>;
    fn try_into_writer(self) -> Result<Self::Raw> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

fn create_file<T: AsRef<Path>>(path: T) -> Result<File> {
    let path = path.as_ref();
    let parent_dir = path.parent();

    if let Some(p) = parent_dir {
        create_dir_all(p)?;
    }

    Ok(File::create(path)?)
}

fn create_file_with_id<T: AsRef<Path>>(path: T, stream_id: &str) -> Result<File> {
    let mut path = PathBuf::from(path.as_ref());

    match path.extension() {
        Some(e) => {
            let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let filename = format!("{file_stem}_{stream_id}.{}", e.to_string_lossy());
            path.set_file_name(filename);
        }
        None => {
            let filename = format!("{stream_id}.las");
            path = path.join(filename);
        }
    };

    create_file(path)
}

impl MakeWriter for PathBuf {
    type Raw = BufWriter<File>;
    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw> {
        let file = create_file_with_id(self, stream_id)?;
        Ok(BufWriter::new(file))
    }
}

impl MakeWriter for String {
    type Raw = BufWriter<File>;
    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw> {
        let file = create_file_with_id(self, stream_id)?;
        Ok(BufWriter::new(file))
    }
}

impl MakeWriter for &'static str {
    type Raw = BufWriter<File>;
    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw> {
        let file = create_file_with_id(self, stream_id)?;
        Ok(BufWriter::new(file))
    }
}

impl MakeWriter for &'static Path {
    type Raw = BufWriter<File>;
    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw> {
        let file = create_file_with_id(self, stream_id)?;
        Ok(BufWriter::new(file))
    }
}

impl<W, F> MakeWriter for F
where
    F: for<'a> Fn(&'a str) -> Result<W> + Clone + Send + 'static,
    W: TryIntoWriter,
{
    type Raw = W::Raw;
    fn make_writer(&self, stream_id: &str) -> Result<Self::Raw> {
        self(stream_id)?.try_into_writer()
    }
}
