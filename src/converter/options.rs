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
