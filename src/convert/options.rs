use std::sync::Arc;

use crate::ext::las::{LasVersion, Scale};

pub type HeaderHook = Arc<dyn Fn(&mut las::Builder) + Send + Sync + 'static>;

const DEFAULT_BATCH_SIZE: usize = 128_000;
const DEFAULT_QUEUE_SIZE: usize = 8;

#[derive(Clone)]
pub struct ConvertOptions {
    pub scale: Scale,
    pub las_version: LasVersion,
    pub header_hook: Option<HeaderHook>,

    #[cfg(feature = "parallel")]
    pub batch_size: usize,

    #[cfg(feature = "parallel")]
    pub queue_size: usize,

    #[cfg(feature = "parallel")]
    pub workers: usize,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            scale: Default::default(),
            las_version: Default::default(),
            header_hook: Default::default(),

            #[cfg(feature = "parallel")]
            batch_size: DEFAULT_BATCH_SIZE,

            #[cfg(feature = "parallel")]
            queue_size: DEFAULT_QUEUE_SIZE,

            #[cfg(feature = "parallel")]
            workers: num_cpus::get(),
        }
    }
}
