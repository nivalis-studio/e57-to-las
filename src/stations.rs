extern crate rayon;

use crate::spatial_point::SpatialPoint;
use anyhow::Result;
use e57::PointCloud;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write as _},
    path::Path,
};

pub(crate) fn save_stations<P: AsRef<Path>>(output_path: P, pointclouds: &[PointCloud]) -> Result<()> {
    let stations: BTreeMap<usize, SpatialPoint> = pointclouds
        .iter()
        .enumerate()
        .map(|(index, pc)| {
            let (x, y, z) = pc
                .transform
                .as_ref()
                .map(|t| (t.translation.x, t.translation.y, t.translation.z))
                .unwrap_or((0.0, 0.0, 0.0));

            let station_point = SpatialPoint { x, y, z };

            (index, station_point)
        })
        .collect();

    let stations_file = File::create(Path::new(&output_path).join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    Ok(())
}
