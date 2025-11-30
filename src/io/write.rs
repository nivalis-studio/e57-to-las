use std::{
    fs::{File, create_dir_all},
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

impl WriterOnce for &'static str {
    type Writer = BufWriter<File>;

    fn try_into_writer(self) -> Result<Self::Writer> {
        let file = File::create(self)?;
        Ok(BufWriter::new(file))
    }
}

impl WriterFactory for &'static str {
    type Writer = BufWriter<File>;
    fn create_writer(&self, ctx: &WriteCtx) -> Result<Self::Writer> {
        let file = create_file_with_id(self, ctx.pc_name.unwrap_or(&ctx.pc_idx.to_string()))?;
        Ok(BufWriter::new(file))
    }
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

    if let Some(p) = path.parent() {
        create_dir_all(p)?;
    }

    Ok(File::create(path)?)
}
