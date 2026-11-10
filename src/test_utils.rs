//! Shared test helpers: fixture lookup, synthetic E57 generation and LAS read-back.

#![allow(clippy::panic, clippy::expect_used)]

use std::path::Path;

use e57::{E57Writer, Quaternion, Record, RecordValue, Transform, Translation};

/// A synthetic point: `(x, y, z, Option<(red, green, blue)>)` with 8-bit colors.
pub(crate) type SyntheticPoint = (f64, f64, f64, Option<(u8, u8, u8)>);

/// A synthetic point cloud. Points of one cloud must either all have color or
/// none of them; `translation` becomes the cloud pose (identity rotation).
pub(crate) struct SyntheticCloud {
    pub(crate) points: Vec<SyntheticPoint>,
    pub(crate) translation: Option<(f64, f64, f64)>,
}

/// Returns the path of the bunny fixture after checking it exists and is not
/// a Git LFS pointer.
pub(crate) fn bunny_fixture() -> String {
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

    input_path
}

/// Writes an E57 file at `path` containing `clouds`, in order.
pub(crate) fn write_synthetic_e57(path: &Path, clouds: &[SyntheticCloud]) {
    let mut writer = E57Writer::from_file(path, "00000000-0000-0000-0000-000000000001")
        .expect("Failed to create E57 writer");

    for (index, cloud) in clouds.iter().enumerate() {
        let has_color = cloud.points.first().is_some_and(|p| p.3.is_some());
        let mut prototype = vec![
            Record::CARTESIAN_X_F64,
            Record::CARTESIAN_Y_F64,
            Record::CARTESIAN_Z_F64,
        ];
        if has_color {
            prototype.extend([
                Record::COLOR_RED_U8,
                Record::COLOR_GREEN_U8,
                Record::COLOR_BLUE_U8,
            ]);
        }

        let guid = format!("00000000-0000-0000-0001-{index:012}");
        let mut pc_writer = writer
            .add_pointcloud(&guid, prototype)
            .expect("Failed to add point cloud");
        pc_writer.set_transform(cloud.translation.map(|(x, y, z)| Transform {
            rotation: Quaternion::default(),
            translation: Translation { x, y, z },
        }));

        for &(x, y, z, color) in &cloud.points {
            let mut values = vec![
                RecordValue::Double(x),
                RecordValue::Double(y),
                RecordValue::Double(z),
            ];
            if has_color {
                let (r, g, b) = color.expect("All points of a colored cloud need a color");
                values.extend([r, g, b].map(|c| RecordValue::Integer(i64::from(c))));
            }
            pc_writer.add_point(values).expect("Failed to add point");
        }
        pc_writer
            .finalize()
            .expect("Failed to finalize point cloud");
    }

    writer.finalize().expect("Failed to finalize E57 file");
}

/// Reads a LAS file back, returning its header and all of its points.
pub(crate) fn read_las(path: &Path) -> (las::Header, Vec<las::Point>) {
    let mut reader = las::Reader::from_path(path)
        .unwrap_or_else(|e| panic!("Failed to open LAS file {}: {e}", path.display()));
    let header = reader.header().clone();
    let points = reader
        .read_all()
        .expect("Failed to read LAS points")
        .points()
        .collect::<las::Result<Vec<_>>>()
        .expect("Failed to decode LAS points");
    (header, points)
}
