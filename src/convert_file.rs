extern crate rayon;
use rayon::prelude::*;

use anyhow::{Context, Result};

use crate::convert_pointcloud::convert_pointcloud;

use crate::stations::save_stations;

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
/// let save_sations = true;
/// convert_file(input_path, output_path, number_of_threads, save_sations);
/// ```
pub fn convert_file(
    input_path: String,
    output_path: String,
    number_of_threads: usize,
    save_sations: bool,
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

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(index, pointcloud)| -> Result<()> {
            println!("Saving pointcloud {}...", index);

            convert_pointcloud(index, pointcloud, &input_path, &output_path)
                .context(format!("Error while converting pointcloud {}", index))?;

            Ok(())
        })
        .context("Error during the parallel processing of pointclouds")?;

    if save_sations {
        save_stations(output_path, pointclouds)?;
    }
    Ok(())
}
