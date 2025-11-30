use std::{
    collections::VecDeque,
    fs::File,
    io::{BufWriter, Cursor, Seek, Write},
};

use crate::Result;

pub trait WriterOnce: Sized {
    type Writer: Write + Seek + Send + 'static;

    fn try_into_writer(self) -> Result<Self::Writer>;
}

pub trait WriterFactory {
    type Writer: Write + Seek + Send + 'static;

    fn create_writer(&self, ctx: &WriteCtx) -> Result<Self::Writer>;
}

pub struct WriteCtx<'a> {
    pub pc_idx: usize,
    pub pc_name: Option<&'a String>,
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
    F: Fn(&WriteCtx) -> Result<W>,
    W: Write + Seek + Send + 'static,
{
    type Writer = W;

    fn create_writer(&self, ctx: &WriteCtx) -> Result<Self::Writer> {
        self(ctx)
    }
}
