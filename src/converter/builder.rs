use crate::las::Scale;

use super::{ConversionOptions, Converter, Merged, Parallel, ParallelOptions, Sequential, Split};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct ConverterBuilder<T = Sequential, S = Merged> {
    opts: ConversionOptions,
    threading_opts: Option<ParallelOptions>,
    _threading: PhantomData<T>,
    _strategy: PhantomData<S>,
}

impl ConverterBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T, S> ConverterBuilder<T, S> {
    pub fn opts(mut self, opts: ConversionOptions) -> Self {
        self.opts = opts;
        self
    }

    pub fn scale(mut self, scale: Scale) -> Self {
        self.opts.scale = scale;
        self
    }

    pub fn las_version(mut self, las_version: (u8, u8)) -> Self {
        self.opts.las_version = las_version;
        self
    }

    pub fn build(self) -> Converter<T, S> {
        Converter {
            opts: self.opts,
            threading_opts: self.threading_opts,
            _threading: PhantomData,
            _strategy: PhantomData,
        }
    }
}

impl<T> ConverterBuilder<T, Merged> {
    pub fn split(&self) -> ConverterBuilder<T, Split> {
        ConverterBuilder {
            opts: self.opts.clone(),
            threading_opts: self.threading_opts.clone(),
            _threading: PhantomData,
            _strategy: PhantomData,
        }
    }
}

impl<S> ConverterBuilder<Parallel, S> {
    pub fn parallel_opts(mut self, opts: ParallelOptions) -> Self {
        self.threading_opts = Some(opts);
        self
    }
}

impl<S> ConverterBuilder<Sequential, S> {
    pub fn parallel(&self) -> ConverterBuilder<Parallel, S> {
        ConverterBuilder {
            opts: self.opts.clone(),
            threading_opts: Default::default(),
            _threading: PhantomData,
            _strategy: PhantomData,
        }
    }
}

impl Default for ConverterBuilder {
    fn default() -> Self {
        Self {
            opts: Default::default(),
            threading_opts: Default::default(),
            _threading: PhantomData,
            _strategy: PhantomData,
        }
    }
}
