extern crate rayon;
use rayon::prelude::*;
use std::path::Path;

use anyhow::{Context, Result};

use crate::convert_pointcloud::{convert_pointcloud, convert_pointclouds};

use crate::LasVersion;
use crate::stations::save_stations;

/// Converts a given e57 file into LAS format and, optionally, as stations.
///
/// This function reads an e57 file, extracts the point clouds, and saves them in single or multiples las files.
/// It can also create stations record file (useful if you use potree).
///
/// # Parameters
/// - `input_path`: The path to the e57 file that needs to be converted.
/// - `output_path`: The destination (output dir) where the files will be saved.
/// - `number_of_threads`: The number of threads to be used for parallel processing.
/// - `as_stations`: Whether to convert e57 file in distinct stations
/// - `las_version`: Version of LAS format used for output file. Latest one is (1, 4). Currently possible: (1, 3) and (1, 4).
/// or in single LAS file
///
/// # Example
/// ```
/// use e57_to_las::{convert_file, LasVersion};
///
/// let input_path = String::from("path/to/input.e57");
/// let output_path = String::from("path/to/output");
/// let number_of_threads = 4;
/// let as_stations = true;
/// let las_version = LasVersion::new(1, 4).expect("Failed to create LAS version");
/// let _ = convert_file(input_path, output_path, number_of_threads, as_stations, las_version);
/// ```
pub fn convert_file(
    input_path: String,
    output_path: String,
    number_of_threads: usize,
    as_stations: bool,
    las_version: LasVersion,
) -> Result<()> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(number_of_threads)
        .build()
        .context("Failed to initialize the thread pool for rayon")?;

    pool.install(|| {
        let e57_reader =
            e57::E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

        if e57_reader.format_name() != "ASTM E57 3D Imaging Data File" {
            return Err(anyhow::anyhow!("Invalid file format"));
        }

        let pointclouds = e57_reader.pointclouds();

        if as_stations {
            pointclouds
                .par_iter()
                .enumerate()
                .try_for_each(|(index, pointcloud)| -> Result<()> {
                    println!("Saving pointcloud {}...", index);

                    convert_pointcloud(
                        index,
                        pointcloud,
                        Path::new(&input_path),
                        Path::new(&output_path),
                        &las_version,
                    )
                    .context(format!("Error while converting pointcloud {}", index))?;

                    Ok(())
                })
                .context("Error during the parallel processing of pointclouds")?;

            save_stations(output_path, &pointclouds)?;
        } else {
            convert_pointclouds(e57_reader, Path::new(&output_path), &las_version)
                .context("Error during the parallel processing of pointclouds")?;
        }
        Ok(())
    })
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use rayon::ThreadPoolBuilder;
    use std::path::Path;

    #[test]
    fn test_convert_bunny() {
        let pool = ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("Failed to build thread pool");
        pool.install(|| {
            let input_path = String::from("examples/bunnyDouble.e57");
            if !Path::new(&input_path).is_file() {
                panic!(
                    "Missing test fixture '{}'. Run `git lfs pull` to retrieve large test files.",
                    input_path
                );
            }

            let is_lfs_pointer = std::fs::read_to_string(&input_path)
                .map(|content| content.starts_with("version https://git-lfs.github.com/spec/v1\n"))
                .unwrap_or(false);

            if is_lfs_pointer {
                panic!(
                    "Test fixture '{}' is a Git LFS pointer. Run `git lfs pull` to retrieve large test files.",
                    input_path
                );
            }

            let output_path = String::from("examples");
            let number_of_threads = 4;
            let as_stations = true;
            let las_version = LasVersion::new(1, 3).expect("Failed to create LAS version");
            let result = convert_file(
                input_path,
                output_path,
                number_of_threads,
                as_stations,
                las_version,
            );

            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_convert_file_twice_in_same_process() {
        let input_path = String::from("examples/bunnyDouble.e57");
        if !Path::new(&input_path).is_file() {
            panic!(
                "Missing test fixture '{}'. Run `git lfs pull` to retrieve large test files.",
                input_path
            );
        }

        let is_lfs_pointer = std::fs::read_to_string(&input_path)
            .map(|content| content.starts_with("version https://git-lfs.github.com/spec/v1\n"))
            .unwrap_or(false);

        if is_lfs_pointer {
            panic!(
                "Test fixture '{}' is a Git LFS pointer. Run `git lfs pull` to retrieve large test files.",
                input_path
            );
        }

        let output_path = String::from("examples");
        let number_of_threads = 4;
        let as_stations = true;
        let las_version = LasVersion::new(1, 3).expect("Failed to create LAS version");

        let first = convert_file(
            input_path.clone(),
            output_path.clone(),
            number_of_threads,
            as_stations,
            las_version,
        );
        assert!(first.is_ok(), "first conversion failed: {:?}", first);

        let las_version = LasVersion::new(1, 3).expect("Failed to create LAS version");
        let second = convert_file(
            input_path,
            output_path,
            number_of_threads,
            as_stations,
            las_version,
        );
        assert!(second.is_ok(), "second conversion failed: {:?}", second);
    }
}
