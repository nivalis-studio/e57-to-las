use std::{
    collections::VecDeque,
    fs::File,
    io::{BufReader, Cursor, Read, Seek},
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
