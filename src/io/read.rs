use std::{
    fs::File,
    io::{BufReader, Read, Seek},
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

impl ReaderOnce for &'static str {
    type Reader = BufReader<File>;

    fn try_into_reader(self) -> Result<Self::Reader> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}

impl ReaderFactory for &'static str {
    type Reader = BufReader<File>;

    fn create_reader(&self) -> Result<Self::Reader> {
        let file = File::open(self)?;
        Ok(BufReader::new(file))
    }
}
