use crate::las::{LasVersion, Scale};

#[derive(Debug, Clone, Default)]
pub struct ConversionOptions {
    pub scale: Scale,
    pub las_version: LasVersion,
}
