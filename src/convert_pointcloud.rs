use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use crate::get_las_writer::get_las_writer;
use crate::{convert_point::convert_point, utils::create_path};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
use las::Write;
use rayon::prelude::*;

/// Converts a point cloud to a LAS file.
///
/// This function take the points from the point cloud, converts them to LAS points using the `convert_point`
/// function, and writes them in a LAS file.
///
/// # Parameters
/// - `index`: The index of the point cloud.
/// - `pointcloud`: A reference to the point cloud to be converted.
/// - `input_path`: A reference to the input file path (E57 file).
/// - `output_path`: A reference to the output dir.
///
/// # Example
/// ```ignore
/// use e57_to_las::convert_pointcloud;
/// let pointcloud = e57::Pointcloud {  };
/// let input_path = String::from("path/to/input.e57");
/// let output_path = String::from("path/to/output");
/// convert_pointcloud(0, &pointcloud, input_path, output_path);
/// ```
pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &String,
    output_path: &String,
) -> Result<()> {
    let path = create_path(
        Path::new(&output_path)
            .join("las")
            .join(format!("{}{}", index, ".las")),
    )
    .context("Unable to create path: ")?;

    let mut writer = get_las_writer(&pointcloud.guid, path).context("Unable to create writer: ")?;

    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        let las_point = match convert_point(point) {
            Some(p) => p,
            None => continue,
        };

        writer.write(las_point).context("Unable to write: ")?;
    }

    writer.close().context("Failed to close the writer: ")?;

    Ok(())
}

/// Converts the pointclouds of an E57Reader to a single LAS file.
///
/// This function takes a E57Reader, reads its pointclouds, converts them to LAS points using the `convert_point`
/// function, and writes them in a single LAS file.
///
/// # Parameters
/// - `e57_reader`: A E57Reader from a file.
/// - `output_path`: A reference to the output dir.
///
/// # Example
/// ```ignore
/// use e57_to_las::convert_pointclouds;
/// let e57_reader = e57::E57Reader::from_file("path/to/input.e56").context("Failed to open e57 file")?;
/// let output_path = String::from("path/to/output");
/// convert_pointclouds(e57_reader, output_path);
/// ```

pub fn convert_pointclouds(
    e57_reader: E57Reader<BufReader<File>>,
    output_path: &String,
) -> Result<()> {
    let path = create_path(
        Path::new(&output_path)
            .join("las")
            .join(format!("{}{}", 0, ".las")),
    )
    .context("Unable to create path: ")?;

    let writer =
        Mutex::new(get_las_writer(e57_reader.guid(), path).context("Unable to create writer: ")?);

    let pointclouds = e57_reader.pointclouds();
    let e57_reader = Mutex::new(e57_reader);

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(index, pointcloud)| -> Result<()> {
            println!("Saving pointclouds {}...", index);

            let mut reader = e57_reader.lock().unwrap();
            let pointcloud_reader = reader
                .pointcloud_simple(pointcloud)
                .context("Unable to get point cloud iterator: ")?;

            for p in pointcloud_reader {
                let point = p.context("Could not read point: ")?;

                let las_point = match convert_point(point) {
                    Some(p) => p,
                    None => continue,
                };

                writer
                    .lock()
                    .unwrap()
                    .write(las_point)
                    .context("Unable to write: ")?;
            }

            Ok(())
        })
        .context("Error during the parallel processing of pointclouds")?;
    writer
        .lock()
        .unwrap()
        .close()
        .context("Failed to close the writer: ")?;
    Ok(())
}
