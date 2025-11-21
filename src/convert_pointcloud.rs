use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::get_las_writer::get_las_writer;
use crate::{LasVersion, convert_point::convert_point, utils::create_path};

use anyhow::{Context, Result};
use e57::{E57Reader, PointCloud};
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
/// use std::path::Path;
/// use e57_to_las::{convert_pointcloud, LasVersion};
///
/// # fn example() -> anyhow::Result<()> {
/// let input_path = Path::new("path/to/input.e57");
/// let output_path = Path::new("path/to/output");
/// let las_version = LasVersion::new(1, 4)?;
/// // pointcloud would be obtained from E57Reader in practice
/// # let pointcloud = todo!();
/// convert_pointcloud(0, &pointcloud, input_path, output_path, &las_version)?;
/// # Ok(())
/// # }
/// ```
pub fn convert_pointcloud(
    index: usize,
    pointcloud: &PointCloud,
    input_path: &Path,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let mut e57_reader = E57Reader::from_file(input_path).context("Failed to open e57 file: ")?;

    let pointcloud_reader = e57_reader
        .pointcloud_simple(pointcloud)
        .context("Unable to get point cloud iterator: ")?;

    let mut max_cartesian: f64 = 0.0;

    let mut las_points: Vec<las::Point> = Vec::new();
    let mut has_color = false;

    for p in pointcloud_reader {
        let point = p.context("Could not read point: ")?;

        if point.color.is_some() {
            has_color = true;
        }

        let las_point = match convert_point(point) {
            Some(p) => p,
            None => continue,
        };

        let abs_extent = las_point
            .x
            .abs()
            .max(las_point.y.abs())
            .max(las_point.z.abs());
        max_cartesian = max_cartesian.max(abs_extent);
        las_points.push(las_point);
    }

    let path = create_path(output_path.join("las").join(format!("{}{}", index, ".las")))
        .context("Unable to create path: ")?;

    let mut writer = get_las_writer(
        pointcloud.clone().guid,
        path,
        max_cartesian,
        has_color,
        las_version,
    )
    .context("Unable to create writer: ")?;

    for p in las_points {
        writer.write_point(p).context("Unable to write: ")?;
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
/// use std::path::Path;
/// use e57_to_las::{convert_pointclouds, LasVersion};
/// use anyhow::Context;
///
/// # fn example() -> anyhow::Result<()> {
/// let input_path = Path::new("path/to/input.e57");
/// let e57_reader = e57::E57Reader::from_file(input_path).context("Failed to open e57 file")?;
/// let output_path = Path::new("path/to/output");
/// let las_version = LasVersion::new(1, 4)?;
/// convert_pointclouds(e57_reader, output_path, &las_version)?;
/// # Ok(())
/// # }
/// ```
pub fn convert_pointclouds(
    e57_reader: E57Reader<BufReader<File>>,
    output_path: &Path,
    las_version: &LasVersion,
) -> Result<()> {
    let pointclouds = e57_reader.pointclouds();
    let guid = &e57_reader.guid().to_owned();
    let e57_reader_mutex = Mutex::new(e57_reader);

    let max_cartesian_mutex = Mutex::new(0.0_f64);
    let las_points: Vec<las::Point> = Vec::new();
    let las_points_mutex = Mutex::new(las_points);
    let has_color = Arc::new(AtomicBool::new(false));

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(index, pointcloud)| -> Result<()> {
            println!("Saving pointclouds {index}...");

            let mut reader = e57_reader_mutex.lock().unwrap_or_else(|e| e.into_inner());
            let pointcloud_reader = reader
                .pointcloud_simple(pointcloud)
                .context("Unable to get point cloud iterator: ")?;

            let mut local_max_cartesian: f64 = 0.0;
            for p in pointcloud_reader {
                let point = p.context("Could not read point: ")?;

                if point.color.is_some() {
                    has_color.store(true, Ordering::Relaxed);
                }

                let las_point = match convert_point(point) {
                    Some(p) => p,
                    None => continue,
                };

                let abs_extent = las_point
                    .x
                    .abs()
                    .max(las_point.y.abs())
                    .max(las_point.z.abs());
                local_max_cartesian = local_max_cartesian.max(abs_extent);
                las_points_mutex
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(las_point);
            }

            let mut guard = max_cartesian_mutex
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let current_max_cartesian = (*guard).max(local_max_cartesian);
            *guard = current_max_cartesian;

            Ok(())
        })
        .context("Error while converting pointcloud")?;

    let path = create_path(output_path.join("las").join(format!("{}{}", 0, ".las")))
        .context("Unable to create path: ")?;

    let max_cartesian = *max_cartesian_mutex
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    let mut writer = get_las_writer(
        Some(guid.to_owned()),
        path,
        max_cartesian,
        has_color.load(Ordering::Relaxed),
        las_version,
    )
    .context("Unable to create writer: ")?;

    let las_points = las_points_mutex
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    for p in las_points {
        writer.write_point(p).context("Unable to write: ")?;
    }

    writer.close().context("Failed to close the writer: ")?;
    Ok(())
}
