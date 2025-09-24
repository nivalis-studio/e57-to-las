use crate::{MakeReader, Reader, Result};

pub trait E57ReaderExt<MR: MakeReader + 'static>
where
    Self: Sized,
{
    fn from_maker(factory: MR) -> Result<Self>;
}

impl<MR, R> E57ReaderExt<MR> for e57::E57Reader<R>
where
    R: Reader,
    MR: 'static + MakeReader<Raw = R>,
{
    fn from_maker(maker: MR) -> Result<e57::E57Reader<R>> {
        let reader = maker.make_reader()?;

        Ok(e57::E57Reader::new(reader)?)
    }
}
