use std::io::{Read, Seek};

use crate::{Result, io::ReaderFactory};

pub trait E57ReaderExt<F: ReaderFactory + 'static>
where
    Self: Sized,
{
    fn from_factory(factory: F) -> Result<Self>;
}

impl<F, R> E57ReaderExt<F> for e57::E57Reader<R>
where
    R: Read + Seek + Send + 'static,
    F: 'static + ReaderFactory<Reader = R>,
{
    fn from_factory(maker: F) -> Result<e57::E57Reader<R>> {
        let reader = maker.create_reader()?;

        Ok(e57::E57Reader::new(reader)?)
    }
}
