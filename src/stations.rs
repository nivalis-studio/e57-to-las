extern crate rayon;

use anyhow::Result;
use e57::{PointCloud, Translation};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct StationPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Serialize)]
pub struct Stations(HashMap<usize, StationPosition>);

impl Stations {
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let stations_file = File::create(path)?;
        let mut writer = BufWriter::new(stations_file);
        serde_json::to_writer(&mut writer, self)?;

        writer.flush()?;

        Ok(())
    }
}

impl From<Vec<PointCloud>> for Stations {
    fn from(value: Vec<PointCloud>) -> Self {
        let stations_map = value
            .iter()
            .enumerate()
            .map(|(index, pc)| {
                let transform = pc.transform.clone().unwrap_or_default();

                let station_position = StationPosition::from(transform.translation);

                (index, station_position)
            })
            .collect();

        Self(stations_map)
    }
}

impl From<Translation> for StationPosition {
    fn from(Translation { x, y, z }: Translation) -> Self {
        Self { x, y, z }
    }
}
