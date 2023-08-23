extern crate rayon;

use crate::spatial_point::SpatialPoint;
use anyhow::Result;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write as IoWrite},
    path::Path,
    sync::Mutex,
};

pub(crate) fn create_station_file(
    output_path: String,
    stations: Mutex<HashMap<usize, SpatialPoint>>,
) -> Result<()> {
    let stations_file = File::create(Path::new(&output_path).join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    Ok(())
}
