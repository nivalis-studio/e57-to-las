use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::Context;
use rayon::prelude::*;

use crate::convert_pointcloud::{
    PointCounts, check_color_support, convert_pointcloud_counted, convert_pointclouds,
};
use crate::stations::save_stations;
use crate::utils::{las_file_path, stations_file_path};
use crate::{Error, LasVersion, Result};

const E57_FORMAT_NAME: &str = "ASTM E57 3D Imaging Data File";

/// How [`convert`] writes LAS output.
///
/// `threads` is `0` to use one worker per logical CPU. `stations` writes one LAS
/// file per point cloud plus `stations.json`; otherwise every cloud is merged
/// into `las/0.las`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub las_version: LasVersion,
    pub threads: usize,
    pub stations: bool,
}

/// Files written by [`convert`], and how many points were kept or dropped.
///
/// `files` lists each `las/<index>.las` in point-cloud order. In stations mode it
/// also lists `stations.json` last. `points_skipped` counts points whose Cartesian
/// coordinates were invalid. `coarse_scale` is true when a file spans more than
/// ~2,147 km and was therefore written with a scale coarser than 1 mm.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Summary {
    pub files: Vec<PathBuf>,
    pub points_written: u64,
    pub points_skipped: u64,
    pub coarse_scale: bool,
}

/// Converts an E57 file to LAS.
///
/// Reads `input`, checks that the LAS version can store the file's color, and
/// writes into `output_dir`. Nothing is printed: the skipped-point total and
/// the coarse-scale flag are on the returned [`Summary`].
///
/// Each LAS file appears in the output directory only after it has been fully
/// written. If conversion fails, partial LAS files are removed.
///
/// # Example
/// ```no_run
/// use e57_to_las::{LasVersion, Options, convert};
///
/// fn main() -> e57_to_las::Result<()> {
///     let options = Options {
///         las_version: LasVersion::new(1, 4)?,
///         threads: 0, // 0 = one worker per logical CPU
///         stations: true,
///     };
///     let summary = convert("path/to/input.e57", "path/to/output/directory", &options)?;
///     println!(
///         "Wrote {} points ({} skipped)",
///         summary.points_written, summary.points_skipped
///     );
///     Ok(())
/// }
/// ```
pub fn convert(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    options: &Options,
) -> Result<Summary> {
    let input = input.as_ref();
    let output_dir = output_dir.as_ref();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.threads)
        .build()
        .map_err(anyhow::Error::from)?;

    pool.install(|| convert_opened(input, output_dir, options))
}

/// Converts a given e57 file into LAS format and, optionally, as stations.
///
/// Same conversion as [`convert`], with the arguments spelled out and nothing
/// returned on success. Use [`convert`] to get the [`Summary`] and an error
/// that can be matched.
///
/// # Parameters
/// - `input_path`: The path to the e57 file that needs to be converted.
/// - `output_path`: The destination (output dir) where the files will be saved.
/// - `number_of_threads`: The number of threads to be used for parallel processing.
/// - `as_stations`: Whether to write one LAS file per point cloud (stations) instead of a single merged LAS file.
/// - `las_version`: Version of LAS format used for output file. Supported: (1, 0) to (1, 4).
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
) -> anyhow::Result<()> {
    convert(
        input_path,
        output_path,
        &Options {
            las_version,
            threads: number_of_threads,
            stations: as_stations,
        },
    )?;
    Ok(())
}

fn convert_opened(input: &Path, output_dir: &Path, options: &Options) -> Result<Summary> {
    let e57_reader = open_e57(input)?;
    if e57_reader.format_name() != E57_FORMAT_NAME {
        return Err(Error::InvalidFormat {
            path: input.to_path_buf(),
        });
    }

    let pointclouds = e57_reader.pointclouds();
    check_color_support(&pointclouds, &options.las_version)?;

    if options.stations {
        return write_stations(input, output_dir, &pointclouds, &options.las_version);
    }

    drop(pointclouds);
    drop(e57_reader);
    let counts = convert_pointclouds(input, output_dir, &options.las_version)
        .map_err(|err| err.context("Error during the parallel processing of pointclouds"))?;
    Ok(Summary {
        files: vec![las_file_path(output_dir, 0)],
        points_written: counts.written,
        points_skipped: counts.skipped,
        coarse_scale: counts.coarse_scale,
    })
}

fn write_stations(
    input: &Path,
    output_dir: &Path,
    pointclouds: &[e57::PointCloud],
    las_version: &LasVersion,
) -> Result<Summary> {
    let counts = pointclouds
        .par_iter()
        .enumerate()
        .map(|(index, pointcloud)| {
            convert_pointcloud_counted(index, pointcloud, input, output_dir, las_version)
                .map_err(|err| err.context(format!("Error while converting pointcloud {index}")))
        })
        .collect::<Result<Vec<PointCounts>>>()
        .map_err(|err| err.context("Error during the parallel processing of pointclouds"))?;

    save_stations(output_dir, pointclouds).context("Failed to write stations.json")?;

    Ok(Summary {
        files: station_files(output_dir, pointclouds.len()),
        points_written: counts.iter().map(|count| count.written).sum(),
        points_skipped: counts.iter().map(|count| count.skipped).sum(),
        coarse_scale: counts.iter().any(|count| count.coarse_scale),
    })
}

fn station_files(output_dir: &Path, cloud_count: usize) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = (0..cloud_count)
        .map(|index| las_file_path(output_dir, index))
        .collect();
    files.push(stations_file_path(output_dir));
    files
}

fn open_e57(path: &Path) -> Result<e57::E57Reader<BufReader<File>>> {
    match e57::E57Reader::from_file(path) {
        Ok(reader) => Ok(reader),
        Err(err) => Err(open_error(path, err)),
    }
}

/// A missing or unreadable path is [`Error::Open`]. A path that opens but is not
/// valid E57 stays an [`Error::UnexpectedError`] carrying the reader error.
fn open_error(path: &Path, err: e57::Error) -> Error {
    match File::open(path) {
        Err(source) => Error::Open {
            path: path.to_path_buf(),
            source,
        },
        Ok(_) => Error::UnexpectedError(
            anyhow::Error::from(err).context(format!("Failed to open {}", path.display())),
        ),
    }
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
            let las_version = LasVersion::new(1, 3).expect("Failed to create LAS version");
            let result = convert(
                input_path,
                output_dir.path(),
                &Options {
                    las_version,
                    threads: 4,
                    stations: true,
                },
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
        let options = Options {
            las_version: LasVersion::new(1, 3).expect("Failed to create LAS version"),
            threads: 4,
            stations: true,
        };

        let first = convert(&input_path, &output_dir, &options);
        assert!(first.is_ok(), "first conversion failed: {:?}", first);

        let second = convert(&input_path, &output_dir, &options);
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
        assert_xyz_within(points, expected, 0.001);
    }

    fn assert_xyz_within(points: &[las::Point], expected: &[(f64, f64, f64)], tolerance: f64) {
        assert_eq!(points.len(), expected.len());
        for (i, (p, &(x, y, z))) in points.iter().zip(expected).enumerate() {
            assert!(
                (p.x - x).abs() <= tolerance
                    && (p.y - y).abs() <= tolerance
                    && (p.z - z).abs() <= tolerance,
                "Point {i}: got ({}, {}, {}), expected ({x}, {y}, {z})",
                p.x,
                p.y,
                p.z
            );
        }
    }

    /// A point on the 0.001 LAS grid with its color, for order-insensitive comparison.
    type PointKey = (i64, i64, i64, Option<(u16, u16, u16)>);

    fn point_key(x: f64, y: f64, z: f64, color: Option<las::Color>) -> PointKey {
        let mm = |v: f64| (v * 1000.0).round() as i64;
        (mm(x), mm(y), mm(z), color.map(|c| (c.red, c.green, c.blue)))
    }

    /// The points of a LAS file as a sorted multiset of keys.
    fn sorted_keys(points: &[las::Point]) -> Vec<PointKey> {
        let mut keys: Vec<_> = points
            .iter()
            .map(|p| point_key(p.x, p.y, p.z, p.color))
            .collect();
        keys.sort_unstable();
        keys
    }

    /// The sorted multiset of keys a merged conversion of colored `clouds` must
    /// produce: pose applied, colorless points backfilled black.
    fn expected_merged_keys(clouds: &[SyntheticCloud]) -> Vec<PointKey> {
        let mut keys: Vec<_> = clouds
            .iter()
            .flat_map(|cloud| {
                expected_xyz(cloud)
                    .into_iter()
                    .zip(&cloud.points)
                    .map(|((x, y, z), p)| point_key(x, y, z, Some(expected_color(p.3))))
            })
            .collect();
        keys.sort_unstable();
        keys
    }

    /// Three clouds of `n` distinct points each (colored, colorless, colored),
    /// with values exactly representable at the 0.001 LAS scale.
    fn generated_clouds(n: usize) -> Vec<SyntheticCloud> {
        (0..3)
            .map(|c| SyntheticCloud {
                points: (0..n)
                    .map(|i| {
                        let color = (c != 1).then_some((
                            if i % 2 == 0 { 0 } else { 255 },
                            if i % 3 == 0 { 0 } else { 255 },
                            255,
                        ));
                        (i as f64 * 0.25, c as f64, (i % 7) as f64 * 0.5, color)
                    })
                    .collect(),
                translation: (c == 2).then_some((100.0, -50.0, 2.5)),
            })
            .collect()
    }

    /// Writes `clouds` to `<dir>/input.e57` and converts it into `dir`.
    fn convert_synthetic(
        dir: &Path,
        clouds: &[SyntheticCloud],
        as_stations: bool,
        las_version: LasVersion,
    ) -> Result<Summary> {
        convert_synthetic_with_threads(dir, clouds, as_stations, las_version, 2)
    }

    fn convert_synthetic_with_threads(
        dir: &Path,
        clouds: &[SyntheticCloud],
        as_stations: bool,
        las_version: LasVersion,
        number_of_threads: usize,
    ) -> Result<Summary> {
        let input = dir.join("input.e57");
        write_synthetic_e57(&input, clouds);
        convert(
            &input,
            dir,
            &Options {
                las_version,
                threads: number_of_threads,
                stations: as_stations,
            },
        )
    }

    /// Converts `clouds` in merged mode and checks the point count and multiset.
    fn assert_merged_multiset(clouds: &[SyntheticCloud], number_of_threads: usize) {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let version = LasVersion::new(1, 4).expect("valid version");
        convert_synthetic_with_threads(dir.path(), clouds, false, version, number_of_threads)
            .expect("conversion failed");

        let (header, points) = read_las(&dir.path().join("las").join("0.las"));
        let total: usize = clouds.iter().map(|c| c.points.len()).sum();
        assert_eq!(header.number_of_points(), total as u64);
        assert_eq!(points.len(), total);
        assert_eq!(sorted_keys(&points), expected_merged_keys(clouds));
    }

    #[test]
    fn test_merged_mode_single_thread_does_not_deadlock() {
        // Several batches per cloud, on a pool with a single thread.
        assert_merged_multiset(&generated_clouds(10_000), 1);
    }

    #[test]
    fn test_merged_mode_more_points_than_the_queue_holds() {
        // 150,000 points: well over CHANNEL_CAPACITY × BATCH_SIZE (65,536).
        assert_merged_multiset(&generated_clouds(50_000), 2);
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

        // Every point appears once with the pose applied (in any order); colored
        // points keep their color and the colorless cloud is backfilled black.
        assert_eq!(sorted_keys(&points), expected_merged_keys(&clouds));

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
    fn test_stations_mode_empty_file_creates_missing_output_dir() {
        let input_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let input = input_dir.path().join("empty.e57");
        write_synthetic_e57(&input, &[]);

        let output_dir = input_dir.path().join("missing-output");
        assert!(
            !output_dir.exists(),
            "The output directory must not exist before conversion"
        );

        let result = convert(
            &input,
            &output_dir,
            &Options {
                threads: 1,
                stations: true,
                ..Options::default()
            },
        );
        assert!(result.is_ok(), "conversion failed: {result:?}");

        let json = std::fs::read_to_string(output_dir.join("stations.json"))
            .expect("Expected stations.json");
        assert_eq!(json, "{}");
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
        let mut skipped_points: u64 = 0;
        for pointcloud in reader.pointclouds() {
            for p in reader
                .pointcloud_simple(&pointcloud)
                .expect("Failed to read pointcloud")
            {
                let p = p.expect("Failed to read point");
                if matches!(p.cartesian, e57::CartesianCoordinate::Valid { .. }) {
                    valid_points += 1;
                } else {
                    skipped_points += 1;
                }
            }
        }
        assert!(valid_points > 0);

        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let version = LasVersion::new(1, 4).expect("valid version");
        let summary = convert(
            &input_path,
            dir.path(),
            &Options {
                las_version: version,
                threads: 2,
                stations: false,
            },
        )
        .expect("conversion failed");

        let (header, points) = read_las(&dir.path().join("las").join("0.las"));
        assert_eq!(header.number_of_points(), valid_points);
        assert_eq!(points.len() as u64, valid_points);
        assert_eq!(summary.points_written, valid_points);
        assert_eq!(summary.points_skipped, skipped_points);
        assert_eq!(summary.files, vec![dir.path().join("las").join("0.las")]);
    }

    #[test]
    fn test_non_finite_points_are_skipped_and_scale_stays_millimeter() {
        // One NaN and one +inf, on different axes, among finite points.
        let clouds = vec![SyntheticCloud {
            points: vec![
                (0.0, 0.0, 0.0, None),
                (1.5, -2.0, 0.5, None),
                (f64::NAN, 1.0, 2.0, None),
                (3.0, f64::INFINITY, -1.0, None),
                (-3.0, 4.5, -1.0, None),
            ],
            translation: Some((10.0, -20.0, 5.5)),
        }];
        let finite = SyntheticCloud {
            points: clouds[0]
                .points
                .iter()
                .copied()
                .filter(|(x, y, z, _)| x.is_finite() && y.is_finite() && z.is_finite())
                .collect(),
            translation: clouds[0].translation,
        };

        for as_stations in [false, true] {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let version = LasVersion::new(1, 4).expect("valid version");
            convert_synthetic(dir.path(), &clouds, as_stations, version)
                .unwrap_or_else(|e| panic!("stations={as_stations}: conversion failed: {e:?}"));

            let (header, points) = read_las(&dir.path().join("las").join("0.las"));
            let t = header.transforms();
            assert_eq!(
                (t.x.scale, t.y.scale, t.z.scale),
                (0.001, 0.001, 0.001),
                "stations={as_stations}"
            );
            assert_eq!(
                header.number_of_points(),
                finite.points.len() as u64,
                "stations={as_stations}"
            );
            assert_xyz(&points, &expected_xyz(&finite));
        }
    }

    #[test]
    fn test_georeferenced_cloud_keeps_millimeter_scale() {
        let clouds = vec![SyntheticCloud {
            points: vec![
                (0.0, 0.0, 0.0, Some((255, 0, 0))),
                (12.345, -6.789, 1.5, Some((0, 255, 0))),
                (-40.0, 25.25, -3.125, Some((0, 0, 255))),
            ],
            translation: Some((5_000_000.0, 1_000_000.0, 300.0)),
        }];
        for as_stations in [false, true] {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let version = LasVersion::new(1, 4).expect("valid version");
            convert_synthetic(dir.path(), &clouds, as_stations, version)
                .expect("conversion failed");

            let (header, points) = read_las(&dir.path().join("las").join("0.las"));
            let t = header.transforms();
            assert_eq!(
                (t.x.scale, t.y.scale, t.z.scale),
                (0.001, 0.001, 0.001),
                "stations={as_stations}"
            );
            assert_eq!(header.number_of_points(), 3);
            assert_xyz(&points, &expected_xyz(&clouds[0]));
        }
    }

    #[test]
    fn test_stations_fallback_for_extent_over_2147_km() {
        let clouds = vec![SyntheticCloud {
            points: vec![(0.0, 1.0, 2.0, None), (5_000_000.0, 1.0, 2.0, None)],
            translation: None,
        }];
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let version = LasVersion::new(1, 4).expect("valid version");
        let summary =
            convert_synthetic(dir.path(), &clouds, true, version).expect("conversion failed");
        assert!(summary.coarse_scale);

        let (header, points) = read_las(&dir.path().join("las").join("0.las"));
        let scale = header.transforms().x.scale;
        assert!(
            scale > 0.001,
            "x scale {scale} should be coarser than 0.001"
        );
        assert_eq!(header.number_of_points(), 2);
        assert_xyz_within(&points, &expected_xyz(&clouds[0]), scale);
    }

    #[test]
    fn test_merged_fallback_on_late_overflow_writes_each_point_once() {
        let clouds = vec![
            SyntheticCloud {
                points: vec![
                    (0.0, 0.0, 0.0, None),
                    (1.5, -2.0, 0.5, None),
                    (-3.0, 4.5, -1.0, None),
                ],
                translation: None,
            },
            SyntheticCloud {
                points: vec![(2.0, 2.5, 3.0, None), (0.0, 0.0, 5_000_000.0, None)],
                translation: None,
            },
        ];
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let version = LasVersion::new(1, 4).expect("valid version");
        let summary =
            convert_synthetic(dir.path(), &clouds, false, version).expect("conversion failed");
        assert!(summary.coarse_scale);

        let (header, points) = read_las(&dir.path().join("las").join("0.las"));
        let scale = header.transforms().z.scale;
        assert!(
            scale > 0.001,
            "z scale {scale} should be coarser than 0.001"
        );
        assert_eq!(header.number_of_points(), 5);
        let expected: Vec<_> = clouds.iter().flat_map(expected_xyz).collect();
        assert_xyz_within(&points, &expected, scale);
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

    #[test]
    fn test_colored_input_rejected_for_las_without_color() {
        for as_stations in [false, true] {
            for minor in [0, 1] {
                let dir = tempfile::tempdir().expect("Failed to create temp dir");
                let version = LasVersion::new(1, minor).expect("valid version");
                let err = convert_synthetic(dir.path(), &mixed_clouds(), as_stations, version)
                    .expect_err("conversion should fail for colored input");

                assert!(
                    matches!(err, Error::ColorUnsupported { version } if version.to_string() == format!("1.{minor}")),
                    "stations={as_stations}, 1.{minor}: unexpected error: {err:?}"
                );
                let message = err.to_string();
                assert!(
                    message.contains(&format!("LAS 1.{minor} cannot store point colors")),
                    "stations={as_stations}, 1.{minor}: unexpected error: {message}"
                );
                assert!(
                    !dir.path().join("las").exists(),
                    "stations={as_stations}, 1.{minor}: no LAS output should be written"
                );
            }
        }
    }

    #[test]
    fn test_colored_input_converts_with_las_1_2() {
        for as_stations in [false, true] {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let version = LasVersion::new(1, 2).expect("valid version");
            convert_synthetic(dir.path(), &mixed_clouds(), as_stations, version)
                .unwrap_or_else(|e| panic!("stations={as_stations}: conversion failed: {e:?}"));
            assert!(dir.path().join("las").join("0.las").is_file());
        }
    }

    // Flips a byte in a later point-data page so the reader yields several
    // packets and then fails with a checksum error. The XML, stored after the
    // point data, stays intact.
    fn corrupt_later_point_page(path: &Path) {
        const PAGE_SIZE: usize = 1024;
        let mut bytes = std::fs::read(path).expect("read e57");
        let xml_offset = usize::try_from(u64::from_le_bytes(
            bytes[24..32].try_into().expect("xml offset bytes"),
        ))
        .expect("xml offset fits");
        // Two full packets of f64 xyz points are about 130 KB. Past that, a
        // checksum error happens only after those points have been decoded.
        let mut offset = xml_offset * 3 / 4;
        offset -= offset % PAGE_SIZE;
        let minimum = PAGE_SIZE * 160;
        if offset < minimum {
            offset = minimum;
        }
        assert!(
            offset + 64 < xml_offset,
            "point data ends at {xml_offset}, corruption at {offset} would miss it"
        );
        bytes[offset] ^= 0xFF;
        std::fs::write(path, bytes).expect("write e57");
    }

    /// How many points the corrupt file yields before the read fails.
    fn points_before_read_error(path: &Path) -> usize {
        let mut reader = e57::E57Reader::from_file(path).expect("open corrupt e57");
        let pointcloud = reader
            .pointclouds()
            .into_iter()
            .next()
            .expect("point cloud");
        let iterator = reader
            .pointcloud_simple(&pointcloud)
            .expect("point iterator");
        let mut decoded = 0;
        let mut failed = false;
        for point in iterator {
            match point {
                Ok(_) => decoded += 1,
                Err(_) => {
                    failed = true;
                    break;
                }
            }
        }
        assert!(failed, "corrupt file was read without error");
        decoded
    }

    fn assert_no_output_files(dir: &Path) {
        let las_dir = dir.join("las");
        if las_dir.exists() {
            let leftovers: Vec<_> = std::fs::read_dir(&las_dir)
                .expect("read las dir")
                .map(|entry| entry.expect("las dir entry").path())
                .collect();
            assert!(
                leftovers.is_empty(),
                "partial output left behind: {leftovers:?}"
            );
        }
        assert!(
            !dir.join("stations.json").exists(),
            "stations.json should not be written when conversion fails"
        );
    }

    #[test]
    fn test_failed_conversion_leaves_no_las_file() {
        // Several packets, so the corrupt page is not in the first one. Merged
        // mode only sends a batch once BATCH_SIZE points are decoded, so the
        // failure must come after that or the writer never opens a file.
        let clouds = vec![SyntheticCloud {
            points: (0..12_000)
                .map(|i| ((i % 1000) as f64 * 0.01, (i / 1000) as f64, -0.5, None))
                .collect(),
            translation: None,
        }];

        for as_stations in [false, true] {
            let dir = tempfile::tempdir().expect("Failed to create temp dir");
            let input = dir.path().join("input.e57");
            write_synthetic_e57(&input, &clouds);
            corrupt_later_point_page(&input);

            let decoded = points_before_read_error(&input);
            assert!(
                decoded >= crate::convert_pointcloud::BATCH_SIZE,
                "stations={as_stations}: only {decoded} points before the error, \
                 the writer would not have opened a file"
            );

            let version = LasVersion::new(1, 4).expect("valid version");
            let err = convert(
                &input,
                dir.path(),
                &Options {
                    las_version: version,
                    threads: 2,
                    stations: as_stations,
                },
            )
            .expect_err("conversion of a corrupt file should fail");
            let message = format!("{err:#}");
            assert!(
                message.contains("Could not read point"),
                "stations={as_stations}: unexpected error: {message}"
            );
            assert_no_output_files(dir.path());
        }
    }

    #[test]
    fn test_summary_for_merged_and_stations() {
        let clouds = mixed_clouds();
        let total: u64 = clouds.iter().map(|cloud| cloud.points.len() as u64).sum();
        let version = LasVersion::new(1, 4).expect("valid version");

        let merged_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let merged = convert_synthetic(merged_dir.path(), &clouds, false, version)
            .expect("merged conversion failed");
        assert_eq!(
            merged.files,
            vec![merged_dir.path().join("las").join("0.las")]
        );
        assert_eq!(merged.points_written, total);
        assert_eq!(merged.points_skipped, 0);
        assert!(!merged.coarse_scale);
        assert_summary_files_exist(&merged);

        let stations_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let stations = convert_synthetic(stations_dir.path(), &clouds, true, version)
            .expect("stations conversion failed");
        let mut expected: Vec<_> = (0..clouds.len())
            .map(|index| stations_dir.path().join("las").join(format!("{index}.las")))
            .collect();
        expected.push(stations_dir.path().join("stations.json"));
        assert_eq!(stations.files, expected);
        assert_eq!(stations.points_written, total);
        assert_eq!(stations.points_skipped, 0);
        assert_summary_files_exist(&stations);

        let written_on_disk: u64 = stations
            .files
            .iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "las"))
            .map(|path| read_las(path).0.number_of_points())
            .sum();
        assert_eq!(written_on_disk, stations.points_written);
    }

    fn assert_summary_files_exist(summary: &Summary) {
        for path in &summary.files {
            assert!(path.is_file(), "missing {}", path.display());
        }
    }

    #[test]
    fn test_missing_file_is_not_found() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let err = convert(
            dir.path().join("missing.e57"),
            dir.path(),
            &Options::default(),
        )
        .expect_err("missing file should fail");

        match err {
            Error::Open { source, .. } => {
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected Error::Open, got {other:?}"),
        }
        assert!(!dir.path().join("las").exists());
    }

    #[test]
    fn test_unreadable_e57_is_not_a_missing_file() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let input = dir.path().join("bad.e57");
        std::fs::write(&input, b"this is not an e57 file").expect("write");

        let err = convert(&input, dir.path(), &Options::default())
            .expect_err("invalid bytes should fail");
        assert!(
            !matches!(err, Error::Open { .. }),
            "a file that exists is not a missing-file error: {err:?}"
        );
    }
}
