use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::{Path, PathBuf},
};

use crate::Result;

pub trait ReaderOnce: Sized {
    type Reader: Read + Seek + Send + 'static;

    fn try_into_reader(self) -> Result<Self::Reader>;
}

pub trait ReaderFactory {
    type Reader: Read + Seek + Send + 'static;

    fn create_reader(&self) -> Result<Self::Reader>;
}

impl ReaderOnce for File {
    type Reader = BufReader<Self>;

    fn try_into_reader(self) -> Result<Self::Reader> {
        Ok(BufReader::new(self))
    }
}

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
    };
}

impl_pathlike!(&Path);
impl_pathlike!(PathBuf);
impl_pathlike!(&PathBuf);
impl_pathlike!(&str);
impl_pathlike!(String);
impl_pathlike!(&String);
