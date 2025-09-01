extern crate rayon;

use crate::spatial_point::SpatialPoint;
use anyhow::Result;
use e57::PointCloud;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write as IoWrite},
    path::Path,
};

pub(crate) fn save_stations(output_path: String, pointclouds: Vec<PointCloud>) -> Result<()> {
    let stations: HashMap<usize, SpatialPoint> = pointclouds
        .iter()
        .enumerate()
        .map(|(index, pc)| {
            let translation = pc
                .transform
                .as_ref()
                .map(|t| &t.translation)
                .unwrap_or(&e57::Translation {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                });

            let station_point = SpatialPoint {
                x: translation.x,
                y: translation.y,
                z: translation.z,
            };

            (index, station_point)
        })
        .collect();

    let stations_file = File::create(Path::new(&output_path).join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    Ok(())
}
