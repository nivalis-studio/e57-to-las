use serde::Serialize;

#[derive(Serialize)]
pub struct StationPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub fn create_station_point(sum_coordinate: (f64, f64, f64), count: f64) -> StationPoint {
    StationPoint {
        x: sum_coordinate.0 / count,
        y: sum_coordinate.1 / count,
        z: sum_coordinate.2 / count,
    }
}

pub fn get_sum_coordinates(
    mut sum_coordinate: (f64, f64, f64),
    point: &e57::Point,
) -> (f64, f64, f64) {
    sum_coordinate = (
        sum_coordinate.0 + point.cartesian.x,
        sum_coordinate.1 + point.cartesian.y,
        sum_coordinate.2 + point.cartesian.z,
    );

    sum_coordinate
}
