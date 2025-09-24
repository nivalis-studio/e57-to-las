use crate::las::{LasVersion, Scale};

use super::{ConversionOptions, Converter};

#[derive(Debug, Default, Clone)]
pub struct ConverterBuilder {
    opts: Option<ConversionOptions>,
    #[cfg(feature = "parallel")]
    workers: Option<usize>,
}

impl ConverterBuilder {
    pub fn new() -> Self {
        ConverterBuilder::default()
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.opts.get_or_insert_default().scale = scale;
        self
    }

    pub fn las_version(mut self, las_version: LasVersion) -> Self {
        self.opts.get_or_insert_default().las_version = las_version;
        self
    }

    #[cfg(feature = "parallel")]
    pub fn workers(mut self, n: usize) -> Self {
        self.workers = Some(n);
        self
    }

    pub fn build(self) -> Converter {
        Converter {
            opts: self.opts.unwrap_or_default(),
            #[cfg(feature = "parallel")]
            workers: self.workers.unwrap_or(num_cpus::get()),
        }
    }
}
