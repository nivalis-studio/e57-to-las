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
                    println!("Saving pointcloud {index}...");

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
            convert_pointclouds(
                Path::new(&input_path),
                Path::new(&output_path),
                &las_version,
            )
            .context("Error during the parallel processing of pointclouds")?;
        }
        Ok(())
    })
}

#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::test_utils::{SyntheticCloud, bunny_fixture, read_las, write_synthetic_e57};
    use rayon::ThreadPoolBuilder;

    #[test]
    fn test_convert_bunny() {
        let pool = ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .expect("Failed to build thread pool");
        pool.install(|| {
            let input_path = bunny_fixture();

            let output_dir = tempfile::tempdir().expect("Failed to create temp output dir");
            let output_path = output_dir
                .path()
                .to_str()
                .expect("Temp dir path is not valid UTF-8")
                .to_string();
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

            let stations_path = output_dir.path().join("stations.json");
            assert!(
                stations_path.is_file(),
                "Expected stations.json to be created in the output dir"
            );

            let las_dir = output_dir.path().join("las");
            let has_las_file = std::fs::read_dir(&las_dir)
                .expect("Expected las directory to be created in the output dir")
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.path().extension().is_some_and(|ext| ext == "las"));
            assert!(
                has_las_file,
                "Expected at least one .las file in the las output dir"
            );
        });
    }

    #[test]
    fn test_convert_file_twice_in_same_process() {
        let input_path = bunny_fixture();

        // Write outside the repo and away from the other test's output so the
        // two conversions in this test only race each other (which is fine —
        // proving repeated calls work is the point of this test).
        let output_dir =
            std::env::temp_dir().join(format!("e57_to_las_twice_{}", std::process::id()));
        let output_path = output_dir
            .to_str()
            .expect("Temp dir path is not valid UTF-8")
            .to_string();
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

        let _ = std::fs::remove_dir_all(&output_dir);
    }

    /// Two clouds: the first colored, translated and with 3 points, the second
    /// colorless, untranslated and with 2 points. Values are exactly
    /// representable at the 0.001 LAS scale.
    fn mixed_clouds() -> Vec<SyntheticCloud> {
        vec![
            SyntheticCloud {
                points: vec![
                    (0.0, 0.0, 0.0, Some((255, 0, 255))),
                    (1.5, -2.0, 0.5, Some((0, 255, 0))),
                    (-3.0, 4.5, -1.0, Some((255, 255, 255))),
                ],
                translation: Some((10.0, -20.0, 5.5)),
            },
            SyntheticCloud {
                points: vec![(2.0, 2.5, 3.0, None), (-0.5, -1.5, 7.0, None)],
                translation: None,
            },
        ]
    }

    /// Expected output coordinates of a cloud: the E57 pose (translation) is
    /// applied by the reader, so each point is shifted by the translation.
    fn expected_xyz(cloud: &SyntheticCloud) -> Vec<(f64, f64, f64)> {
        let (tx, ty, tz) = cloud.translation.unwrap_or((0.0, 0.0, 0.0));
        cloud
            .points
            .iter()
            .map(|&(x, y, z, _)| (x + tx, y + ty, z + tz))
            .collect()
    }

    /// Converts the E57 8-bit color to the expected LAS 16-bit color (exact for 0 and 255).
    fn expected_color(color: Option<(u8, u8, u8)>) -> las::Color {
        color.map_or_else(las::Color::default, |(r, g, b)| {
            las::Color::new(u16::from(r) * 257, u16::from(g) * 257, u16::from(b) * 257)
        })
    }

    fn assert_xyz(points: &[las::Point], expected: &[(f64, f64, f64)]) {
        assert_eq!(points.len(), expected.len());
        for (i, (p, &(x, y, z))) in points.iter().zip(expected).enumerate() {
            assert!(
                (p.x - x).abs() <= 0.001 && (p.y - y).abs() <= 0.001 && (p.z - z).abs() <= 0.001,
                "Point {i}: got ({}, {}, {}), expected ({x}, {y}, {z})",
                p.x,
                p.y,
                p.z
            );
        }
    }

    /// Writes `clouds` to `<dir>/input.e57` and converts it into `dir`.
    fn convert_synthetic(
        dir: &Path,
        clouds: &[SyntheticCloud],
        as_stations: bool,
        las_version: LasVersion,
    ) -> Result<()> {
        let input = dir.join("input.e57");
        write_synthetic_e57(&input, clouds);
        convert_file(
            input.to_str().expect("non UTF-8 path").to_string(),
            dir.to_str().expect("non UTF-8 path").to_string(),
            2,
            as_stations,
            las_version,
        )
    }

    #[test]
    fn test_merged_mode_reads_back_points_and_colors() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let clouds = mixed_clouds();
        let version = LasVersion::new(1, 4).expect("valid version");
        convert_synthetic(dir.path(), &clouds, false, version).expect("conversion failed");

        let las_path = dir.path().join("las").join("0.las");
        assert!(las_path.is_file(), "Expected las/0.las");
        let (header, points) = read_las(&las_path);

        let total: usize = clouds.iter().map(|c| c.points.len()).sum();
        assert_eq!(points.len(), total);
        assert_eq!(header.number_of_points(), total as u64);

        // Points appear in cloud order, with the pose applied.
        let expected: Vec<_> = clouds.iter().flat_map(expected_xyz).collect();
        assert_xyz(&points, &expected);

        // Colored points keep their color; the colorless cloud is backfilled black.
        let expected_colors: Vec<_> = clouds
            .iter()
            .flat_map(|c| c.points.iter().map(|p| Some(expected_color(p.3))))
            .collect();
        let colors: Vec<_> = points.iter().map(|p| p.color).collect();
        assert_eq!(colors, expected_colors);
        assert_eq!(points[3].color, Some(las::Color::default()));

        let bounds = header.bounds();
        for p in &points {
            assert!(
                bounds.min.x <= p.x && p.x <= bounds.max.x,
                "x out of bounds"
            );
            assert!(
                bounds.min.y <= p.y && p.y <= bounds.max.y,
                "y out of bounds"
            );
            assert!(
                bounds.min.z <= p.z && p.z <= bounds.max.z,
                "z out of bounds"
            );
        }
    }

    #[test]
    fn test_stations_mode_reads_back_each_cloud_and_stations() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let clouds = mixed_clouds();
        let version = LasVersion::new(1, 4).expect("valid version");
        convert_synthetic(dir.path(), &clouds, true, version).expect("conversion failed");

        for (i, cloud) in clouds.iter().enumerate() {
            let (header, points) = read_las(&dir.path().join("las").join(format!("{i}.las")));
            assert_eq!(header.number_of_points(), cloud.points.len() as u64);
            assert_xyz(&points, &expected_xyz(cloud));

            // Each station file only carries color if its own cloud has color.
            let colors: Vec<_> = points.iter().map(|p| p.color).collect();
            let expected_colors: Vec<_> = cloud
                .points
                .iter()
                .map(|p| p.3.map(|c| expected_color(Some(c))))
                .collect();
            assert_eq!(colors, expected_colors);
        }

        let json = std::fs::read_to_string(dir.path().join("stations.json"))
            .expect("Expected stations.json");
        let stations: serde_json::Value = serde_json::from_str(&json).expect("invalid JSON");
        let stations = stations
            .as_object()
            .expect("stations.json is not an object");
        assert_eq!(stations.len(), clouds.len());
        for (i, cloud) in clouds.iter().enumerate() {
            let station = &stations[&i.to_string()];
            let (x, y, z) = cloud.translation.unwrap_or((0.0, 0.0, 0.0));
            assert_eq!(station["x"].as_f64(), Some(x), "station {i} x");
            assert_eq!(station["y"].as_f64(), Some(y), "station {i} y");
            assert_eq!(station["z"].as_f64(), Some(z), "station {i} z");
        }
    }

    #[test]
    fn test_merged_mode_bunny_keeps_all_valid_points() {
        let input_path = bunny_fixture();

        let mut reader = e57::E57Reader::from_file(&input_path).expect("Failed to open bunny");
        let mut valid_points: u64 = 0;
        for pointcloud in reader.pointclouds() {
            for p in reader
                .pointcloud_simple(&pointcloud)
                .expect("Failed to read pointcloud")
            {
                let p = p.expect("Failed to read point");
                if matches!(p.cartesian, e57::CartesianCoordinate::Valid { .. }) {
                    valid_points += 1;
                }
            }
        }
        assert!(valid_points > 0);

        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let version = LasVersion::new(1, 4).expect("valid version");
        convert_file(
            input_path,
            dir.path().to_str().expect("non UTF-8 path").to_string(),
            2,
            false,
            version,
        )
        .expect("conversion failed");

        let (header, points) = read_las(&dir.path().join("las").join("0.las"));
        assert_eq!(header.number_of_points(), valid_points);
        assert_eq!(points.len() as u64, valid_points);
    }

    #[test]
    fn test_colorless_conversion_for_every_las_version() {
        let clouds = vec![SyntheticCloud {
            points: vec![(1.0, 2.0, 3.0, None), (-1.5, 0.5, 2.0, None)],
            translation: Some((100.0, 200.0, 0.5)),
        }];
        for minor in 0..=4 {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let version = LasVersion::new(1, minor).expect("valid version");
            convert_synthetic(dir.path(), &clouds, false, version)
                .unwrap_or_else(|e| panic!("LAS 1.{minor} conversion failed: {e:?}"));

            let (header, points) = read_las(&dir.path().join("las").join("0.las"));
            let v = header.version();
            assert_eq!((v.major, v.minor), (1, minor));
            assert_xyz(&points, &expected_xyz(&clouds[0]));
        }
    }
}
