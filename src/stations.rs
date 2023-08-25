#[cfg(feature = "stations")]
extern crate rayon;

#[cfg(feature = "stations")]
use crate::spatial_point::SpatialPoint;
#[cfg(feature = "stations")]
use anyhow::Result;
#[cfg(feature = "stations")]
use serde_json;
#[cfg(feature = "stations")]
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write as IoWrite},
    path::Path,
};

#[cfg(feature = "stations")]
pub(crate) fn create_station_file(
    output_path: String,
    stations: HashMap<usize, SpatialPoint>,
) -> Result<()> {
    let stations_file = File::create(Path::new(&output_path).join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    Ok(())
}
