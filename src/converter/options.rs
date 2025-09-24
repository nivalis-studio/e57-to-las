use std::thread;

use crate::las::Scale;

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub scale: Scale,
    pub las_version: (u8, u8),
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            scale: Default::default(),
            las_version: (1, 4),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParallelOptions {
    pub workers: usize,
}

impl Default for ParallelOptions {
    fn default() -> Self {
        let workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        Self { workers }
    }
}
