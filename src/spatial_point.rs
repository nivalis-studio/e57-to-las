#[cfg(feature = "stations")]
use serde::Serialize;

#[cfg_attr(feature = "stations", derive(Serialize))]
#[derive(Debug)]
pub struct SpatialPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
