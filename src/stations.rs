use serde::Serialize;

#[derive(Serialize)]
pub struct StationPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
