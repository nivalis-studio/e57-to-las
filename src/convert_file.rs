extern crate rayon;
use rayon::prelude::*;

use anyhow::{Context, Result};
use std::{collections::HashMap, sync::Mutex};

use crate::convert_pointcloud::convert_pointcloud;
use crate::spatial_point::SpatialPoint;
use crate::stations::create_station_file;

/// Converts a given e57 file into a series of point clouds and station files.
///
/// This function reads an e57 file, extracts the point clouds, and saves them in multiples las files.
/// It also creates station files from the point clouds, if you use potree.
///
/// # Parameters
/// - `input_path`: The path to the e57 file that needs to be converted.
/// - `output_path`: The destination (output dir) where the files will be saved.
/// - `number_of_threads`: The number of threads to be used for parallel processing.
///
/// # Example
/// ```
/// use e57_to_las::convert_file;
/// let input_path = String::from("path/to/input.e57");
/// let output_path = String::from("path/to/output");
/// let number_of_threads = 4;
/// convert_file(input_path, output_path, number_of_threads);
/// ```
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
    let stations: Mutex<HashMap<usize, SpatialPoint>> = Mutex::new(HashMap::new());

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(index, pointcloud)| -> Result<()> {
            println!("Saving pointcloud {}...", index);

            let converter_result = convert_pointcloud(index, pointcloud, &input_path, &output_path)
                .context(format!("Error while converting pointcloud {}", index))?;

            stations
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock stations"))?
                .extend(converter_result);

            Ok(())
        })
        .context("Error during the parallel processing of pointclouds")?;

    create_station_file(output_path, stations)?;

    Ok(())
}
