use std::{
    fs::File,
    io::{BufWriter, Seek, Write},
    path::{Path, PathBuf},
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

macro_rules! impl_pathlike {
    ($t:ty) => {
        impl WriterOnce for $t {
            type Writer = BufWriter<File>;

            fn try_into_writer(self) -> Result<Self::Writer> {
                let path: &Path = self.as_ref();
                let file = File::open(path)?;
                Ok(BufWriter::new(file))
            }
        }

        impl WriterFactory for $t {
            type Writer = BufWriter<File>;

            fn create_writer(&self, ctx: &WriteCtx) -> Result<Self::Writer> {
                let mut path = PathBuf::from(self);
                let cloud_id = match ctx.pc_name {
                    Some(n) => n.to_owned(),
                    None => ctx.pc_idx.to_string(),
                };

                match path.extension() {
                    Some(e) => {
                        let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
                        let filename = format!("{file_stem}_{cloud_id}.{}", e.to_string_lossy());
                        path.set_file_name(filename);
                    }
                    None => {
                        let filename = format!("{cloud_id}.las");
                        path = path.join(filename);
                    }
                };

                let file = File::create(path)?;

                Ok(BufWriter::new(file))
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
