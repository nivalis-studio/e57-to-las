use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SpatialPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
