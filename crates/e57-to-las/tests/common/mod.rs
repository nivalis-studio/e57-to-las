//! Synthetic E57 files and LAS read-back for the integration tests.

#![allow(
    dead_code,
    unreachable_pub,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use e57::{
    CartesianBounds, DateTime, E57Writer, Extension, Quaternion, Record, RecordDataType,
    RecordName, RecordValue, Transform, Translation,
};

/// One synthetic point. Unset fields are absent from the record.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct P {
    pub xyz: [f64; 3],
    /// Writes the point with an invalid Cartesian state.
    pub invalid: bool,
    pub color: Option<[u8; 3]>,
    pub intensity: Option<u16>,
    pub time: Option<f64>,
    /// `(return index from 0, return count)`.
    pub returns: Option<(i64, i64)>,
    pub normal: Option<[f32; 3]>,
    pub row_column: Option<(i64, i64)>,
}

pub fn p(x: f64, y: f64, z: f64) -> P {
    P {
        xyz: [x, y, z],
        ..P::default()
    }
}

impl P {
    pub fn color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.color = Some([r, g, b]);
        self
    }
    pub fn intensity(mut self, i: u16) -> Self {
        self.intensity = Some(i);
        self
    }
    pub fn time(mut self, t: f64) -> Self {
        self.time = Some(t);
        self
    }
    pub fn returns(mut self, index: i64, count: i64) -> Self {
        self.returns = Some((index, count));
        self
    }
    pub fn normal(mut self, n: [f32; 3]) -> Self {
        self.normal = Some(n);
        self
    }
    pub fn row_column(mut self, row: i64, column: i64) -> Self {
        self.row_column = Some((row, column));
        self
    }
    pub fn invalid(mut self) -> Self {
        self.invalid = true;
        self
    }
}

/// One synthetic point cloud. Every point must have the same set of fields.
#[derive(Debug, Clone, Default)]
pub struct Cloud {
    pub name: Option<String>,
    pub points: Vec<P>,
    pub translation: Option<[f64; 3]>,
    /// Quaternion `[w, x, y, z]`.
    pub rotation: Option<[f64; 4]>,
    pub acquisition_start: Option<f64>,
    pub sensor_model: Option<String>,
    /// Overrides the Cartesian bounds metadata (in local coordinates).
    pub bounds: Option<([f64; 3], [f64; 3])>,
}

pub fn cloud(points: Vec<P>) -> Cloud {
    Cloud {
        points,
        ..Cloud::default()
    }
}

impl Cloud {
    pub fn named(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }
    pub fn translated(mut self, t: [f64; 3]) -> Self {
        self.translation = Some(t);
        self
    }
    pub fn rotated(mut self, q: [f64; 4]) -> Self {
        self.rotation = Some(q);
        self
    }
    pub fn started_at(mut self, gps_time: f64) -> Self {
        self.acquisition_start = Some(gps_time);
        self
    }
    pub fn model(mut self, model: &str) -> Self {
        self.sensor_model = Some(model.to_owned());
        self
    }
    pub fn bounds(mut self, min: [f64; 3], max: [f64; 3]) -> Self {
        self.bounds = Some((min, max));
        self
    }
}

const NOR: &str = "nor";

fn unknown(name: &str) -> RecordName {
    RecordName::Unknown {
        namespace: NOR.to_owned(),
        name: name.to_owned(),
    }
}

/// Writes `clouds` to an E57 file at `path`.
pub fn write_e57(path: &Path, clouds: &[Cloud]) {
    write_e57_with(path, clouds, None);
}

/// Like [`write_e57`], with E57 `coordinateMetadata`.
pub fn write_e57_with(path: &Path, clouds: &[Cloud], coordinate_metadata: Option<&str>) {
    let mut writer = E57Writer::from_file(path, "00000000-0000-0000-0000-000000000001")
        .expect("create E57 writer");
    writer.set_coordinate_metadata(coordinate_metadata.map(str::to_owned));
    if clouds
        .iter()
        .flat_map(|c| &c.points)
        .any(|p| p.normal.is_some())
    {
        writer
            .register_extension(Extension::new(
                NOR,
                "http://www.libe57.org/E57_NOR_surface_normals.txt",
            ))
            .expect("register extension");
    }

    for (index, cloud) in clouds.iter().enumerate() {
        let any = |f: fn(&P) -> bool| cloud.points.iter().any(f);
        let mut prototype = vec![
            Record::CARTESIAN_X_F64,
            Record::CARTESIAN_Y_F64,
            Record::CARTESIAN_Z_F64,
        ];
        let invalid = any(|p| p.invalid);
        let color = any(|p| p.color.is_some());
        let intensity = any(|p| p.intensity.is_some());
        let time = any(|p| p.time.is_some());
        let returns = any(|p| p.returns.is_some());
        let normals = any(|p| p.normal.is_some());
        let row_column = any(|p| p.row_column.is_some());
        let int = |min, max| RecordDataType::Integer { min, max };
        if invalid {
            prototype.push(Record::CARTESIAN_INVALID_STATE);
        }
        if color {
            prototype.extend([
                Record::COLOR_RED_U8,
                Record::COLOR_GREEN_U8,
                Record::COLOR_BLUE_U8,
            ]);
        }
        if intensity {
            prototype.push(Record::INTENSITY_U16);
        }
        if time {
            prototype.push(Record {
                name: RecordName::TimeStamp,
                data_type: RecordDataType::F64,
            });
        }
        if returns {
            prototype.push(Record {
                name: RecordName::ReturnIndex,
                data_type: int(0, 15),
            });
            prototype.push(Record {
                name: RecordName::ReturnCount,
                data_type: int(0, 15),
            });
        }
        if normals {
            for name in ["normalX", "normalY", "normalZ"] {
                prototype.push(Record {
                    name: unknown(name),
                    data_type: RecordDataType::F32,
                });
            }
        }
        if row_column {
            prototype.push(Record {
                name: RecordName::RowIndex,
                data_type: int(0, 1 << 20),
            });
            prototype.push(Record {
                name: RecordName::ColumnIndex,
                data_type: int(0, 1 << 20),
            });
        }

        let guid = format!("00000000-0000-0000-0001-{index:012}");
        let mut pc = writer
            .add_pointcloud(&guid, prototype)
            .expect("add point cloud");
        pc.set_name(cloud.name.clone());
        pc.set_sensor_model(cloud.sensor_model.clone());
        if cloud.translation.is_some() || cloud.rotation.is_some() {
            let [x, y, z] = cloud.translation.unwrap_or_default();
            let [qw, qx, qy, qz] = cloud.rotation.unwrap_or([1.0, 0.0, 0.0, 0.0]);
            pc.set_transform(Some(Transform {
                rotation: Quaternion {
                    w: qw,
                    x: qx,
                    y: qy,
                    z: qz,
                },
                translation: Translation { x, y, z },
            }));
        }
        pc.set_acquisition_start(cloud.acquisition_start.map(|gps_time| DateTime {
            gps_time,
            atomic_reference: true,
        }));

        for point in &cloud.points {
            let mut values: Vec<RecordValue> =
                point.xyz.iter().map(|v| RecordValue::Double(*v)).collect();
            if invalid {
                values.push(RecordValue::Integer(if point.invalid { 2 } else { 0 }));
            }
            if color {
                let [r, g, b] = point.color.expect("every point needs a color");
                values.extend([r, g, b].map(|c| RecordValue::Integer(i64::from(c))));
            }
            if intensity {
                values.push(RecordValue::Integer(i64::from(
                    point.intensity.expect("intensity"),
                )));
            }
            if time {
                values.push(RecordValue::Double(point.time.expect("time")));
            }
            if returns {
                let (i, n) = point.returns.expect("returns");
                values.extend([RecordValue::Integer(i), RecordValue::Integer(n)]);
            }
            if normals {
                let n = point.normal.expect("normal");
                values.extend(n.map(RecordValue::Single));
            }
            if row_column {
                let (r, c) = point.row_column.expect("row/column");
                values.extend([RecordValue::Integer(r), RecordValue::Integer(c)]);
            }
            pc.add_point(values).expect("add point");
        }
        if let Some((min, max)) = cloud.bounds {
            pc.set_cartesian_bounds(Some(CartesianBounds {
                x_min: Some(min[0]),
                x_max: Some(max[0]),
                y_min: Some(min[1]),
                y_max: Some(max[1]),
                z_min: Some(min[2]),
                z_max: Some(max[2]),
            }));
        }
        pc.finalize().expect("finalize point cloud");
    }
    writer.finalize().expect("finalize E57 file");
}

/// A temporary directory holding `input.e57` made of `clouds`.
pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub input: PathBuf,
}

pub fn fixture(clouds: &[Cloud]) -> Fixture {
    fixture_with(clouds, None)
}

pub fn fixture_with(clouds: &[Cloud], coordinate_metadata: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().expect("temp dir");
    let input = dir.path().join("input.e57");
    write_e57_with(&input, clouds, coordinate_metadata);
    Fixture { dir, input }
}

impl Fixture {
    pub fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    /// Every file under the directory, relative, sorted.
    pub fn files(&self) -> Vec<String> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).expect("read dir") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.push(
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
        }
        let mut out = Vec::new();
        walk(self.dir.path(), self.dir.path(), &mut out);
        out.sort();
        out
    }
}

/// Reads a LAS or LAZ file back.
pub fn read_las(path: &Path) -> (las::Header, Vec<las::Point>) {
    let mut reader = las::Reader::from_path(path)
        .unwrap_or_else(|e| panic!("cannot open {}: {e}", path.display()));
    read_all(&mut reader)
}

pub fn read_las_bytes(bytes: Vec<u8>) -> (las::Header, Vec<las::Point>) {
    let mut reader = las::Reader::new(std::io::Cursor::new(bytes)).expect("read LAS bytes");
    read_all(&mut reader)
}

fn read_all(reader: &mut las::Reader) -> (las::Header, Vec<las::Point>) {
    let header = reader.header().clone();
    let points = reader
        .read_all()
        .expect("read LAS points")
        .points()
        .collect::<las::Result<Vec<_>>>()
        .expect("decode LAS points");
    (header, points)
}

/// Coordinates rounded to the millimeter, for comparisons.
pub fn mm(point: &las::Point) -> [i64; 3] {
    [point.x, point.y, point.z].map(|v| (v * 1000.0).round() as i64)
}

pub fn sorted_mm(points: &[las::Point]) -> Vec<[i64; 3]> {
    let mut out: Vec<_> = points.iter().map(mm).collect();
    out.sort_unstable();
    out
}

/// The repository's bunny fixture, checked to be pulled from Git LFS.
pub fn bunny() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bunnyDouble.e57");
    let head = std::fs::read(&path)
        .unwrap_or_else(|_| panic!("missing {}: run `git lfs pull`", path.display()));
    assert!(
        !head.starts_with(b"version https://git-lfs"),
        "{} is a Git LFS pointer: run `git lfs pull`",
        path.display()
    );
    path
}
