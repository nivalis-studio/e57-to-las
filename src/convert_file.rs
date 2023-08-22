extern crate rayon;
use rayon::prelude::*;

use crate::convert_pointcloud::convert_pointcloud;
use crate::stations::{create_station_file, StationPoint};
use anyhow::{Context, Result};
use std::{collections::HashMap, sync::Mutex};

pub fn convert_file(
    input_path: String,
    output_path: String,
    number_of_threads: usize,
) -> Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(number_of_threads)
        .build_global()
        .context("Failed to initialize the global thread pool")?;

    let e57_reader = e57::E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

    if e57_reader.format_name() != "ASTM E57 3D Imaging Data File" {
        return Err(anyhow::anyhow!("Invalid file format"));
    }

    let pointclouds = e57_reader.pointclouds();
    let stations: Mutex<HashMap<usize, StationPoint>> = Mutex::new(HashMap::new());

    pointclouds
        .par_iter()
        .enumerate()
        .for_each(|(index, pointcloud)| -> () {
            println!("Saving pointcloud {}...", index);
            let converter_result =
                match convert_pointcloud(index, pointcloud, &input_path, &output_path) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error encountered: {}", e);
                        return;
                    }
                };

            stations.lock().unwrap().extend(converter_result);
        });

    create_station_file(output_path, stations)?;

    Ok(())
}
