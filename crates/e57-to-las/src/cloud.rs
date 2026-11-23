//! What an E57 file says about its point clouds, before any point is read.

/// Position and orientation of a point cloud (a scan station) in the file's
/// coordinate system.
///
/// Converted points already have the pose applied. It is exposed so callers
/// can locate stations, as the CLI does with `--stations`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pose {
    /// Rotation as a quaternion `[w, x, y, z]`.
    pub rotation: [f64; 4],
    /// Translation `[x, y, z]`: where the scanner stood.
    pub translation: [f64; 3],
}

impl Pose {
    /// No rotation, no translation.
    pub const IDENTITY: Pose = Pose {
        rotation: [1.0, 0.0, 0.0, 0.0],
        translation: [0.0, 0.0, 0.0],
    };

    /// Applies the pose to a point: rotation, then translation.
    pub fn transform(&self, point: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = self.rotate(point);
        let [tx, ty, tz] = self.translation;
        [x + tx, y + ty, z + tz]
    }

    /// Applies only the rotation, as for a direction or a normal.
    pub fn rotate(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        let m = self.matrix();
        [
            m[0] * x + m[3] * y + m[6] * z,
            m[1] * x + m[4] * y + m[7] * z,
            m[2] * x + m[5] * y + m[8] * z,
        ]
    }

    /// Column-major rotation matrix, computed exactly as the `e57` crate does
    /// when it applies poses, so metadata bounds and decoded points agree.
    pub(crate) fn matrix(&self) -> [f64; 9] {
        let [w, x, y, z] = self.rotation;
        [
            w * w + x * x - y * y - z * z,
            2.0 * (x * y + w * z),
            2.0 * (x * z - w * y),
            2.0 * (x * y - w * z),
            w * w + y * y - x * x - z * z,
            2.0 * (y * z + w * x),
            2.0 * (x * z + w * y),
            2.0 * (y * z - w * x),
            w * w + z * z - x * x - y * y,
        ]
    }
}

impl Default for Pose {
    fn default() -> Self {
        Pose::IDENTITY
    }
}

/// An axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bounds {
    /// Smallest `[x, y, z]`.
    pub min: [f64; 3],
    /// Largest `[x, y, z]`.
    pub max: [f64; 3],
}

impl Bounds {
    /// Bounds holding no point. Adding a point makes them valid.
    pub(crate) const EMPTY: Bounds = Bounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };

    /// Whether no point was ever added.
    pub(crate) fn is_empty(&self) -> bool {
        (0..3).any(|axis| self.min[axis] > self.max[axis])
    }

    pub(crate) fn add(&mut self, point: [f64; 3]) {
        for (axis, value) in point.into_iter().enumerate() {
            self.min[axis] = self.min[axis].min(value);
            self.max[axis] = self.max[axis].max(value);
        }
    }

    pub(crate) fn merge(&mut self, other: &Bounds) {
        self.add(other.min);
        self.add(other.max);
    }

    /// The bounds of these bounds once moved by `pose`.
    fn transformed(&self, pose: &Pose) -> Bounds {
        let mut out = Bounds::EMPTY;
        for corner in 0..8 {
            let pick = |axis: usize| {
                if corner & (1 << axis) == 0 {
                    self.min[axis]
                } else {
                    self.max[axis]
                }
            };
            out.add(pose.transform([pick(0), pick(1), pick(2)]));
        }
        out
    }
}

/// One point cloud of an E57 file, as described by the file's metadata.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct CloudInfo {
    /// Position of the point cloud in the file, from 0.
    pub index: usize,
    /// GUID of the point cloud, if the file has one.
    pub guid: Option<String>,
    /// Name, often the scan station name.
    pub name: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Number of point records, including records with invalid coordinates.
    pub point_count: u64,
    /// Pose of the scanner. Identity when the file has none.
    pub pose: Pose,
    /// Bounds of the points with the pose applied, from the file metadata.
    /// Metadata can be missing or wrong; the converter checks every point.
    pub bounds: Option<Bounds>,
    /// Points have colors.
    pub has_color: bool,
    /// Points have intensities.
    pub has_intensity: bool,
    /// Points have timestamps.
    pub has_time: bool,
    /// Points have return index and count.
    pub has_returns: bool,
    /// Points have surface normals (E57 `nor:` extension).
    pub has_normals: bool,
    /// Points have row and column indices of the scan grid.
    pub has_row_column: bool,
    /// When the scan started, in GPS seconds (since 1980-01-06).
    pub acquisition_start: Option<f64>,
    /// Scanner manufacturer.
    pub sensor_vendor: Option<String>,
    /// Scanner model.
    pub sensor_model: Option<String>,
    /// Scanner serial number.
    pub sensor_serial: Option<String>,
}

pub(crate) const NORMAL_NAMES: [&str; 3] = ["normalX", "normalY", "normalZ"];

impl CloudInfo {
    /// A point cloud with no attribute, for unit tests.
    #[cfg(test)]
    pub(crate) fn for_tests(index: usize) -> Self {
        CloudInfo {
            index,
            guid: None,
            name: None,
            description: None,
            point_count: 0,
            pose: Pose::IDENTITY,
            bounds: None,
            has_color: false,
            has_intensity: false,
            has_time: false,
            has_returns: false,
            has_normals: false,
            has_row_column: false,
            acquisition_start: None,
            sensor_vendor: None,
            sensor_model: None,
            sensor_serial: None,
        }
    }

    pub(crate) fn from_e57(index: usize, pc: &e57::PointCloud) -> Self {
        let pose = pc.transform.as_ref().map_or(Pose::IDENTITY, |t| Pose {
            rotation: [t.rotation.w, t.rotation.x, t.rotation.y, t.rotation.z],
            translation: [t.translation.x, t.translation.y, t.translation.z],
        });
        let bounds = pc.get_cartesian_bounds().and_then(|b| {
            let local = Bounds {
                min: [b.x_min?, b.y_min?, b.z_min?],
                max: [b.x_max?, b.y_max?, b.z_max?],
            };
            let finite = local.min.iter().chain(&local.max).all(|v| v.is_finite());
            (finite && !local.is_empty()).then(|| local.transformed(&pose))
        });
        let has_normals = NORMAL_NAMES.iter().all(|name| {
            pc.prototype
                .iter()
                .any(|r| matches!(&r.name, e57::RecordName::Unknown { name: n, .. } if n == name))
        });

        CloudInfo {
            index,
            guid: pc.guid.clone(),
            name: pc.name.clone(),
            description: pc.description.clone(),
            point_count: pc.records,
            pose,
            bounds,
            has_color: pc.has_color(),
            has_intensity: pc.has_intensity(),
            has_time: pc.has_timestamp(),
            has_returns: pc.has_return(),
            has_normals,
            has_row_column: pc.has_row_column(),
            acquisition_start: pc.acquisition_start.as_ref().map(|d| d.gps_time),
            sensor_vendor: pc.sensor_vendor.clone(),
            sensor_model: pc.sensor_model.clone(),
            sensor_serial: pc.sensor_serial.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn identity_pose_keeps_points() {
        assert_eq!(Pose::IDENTITY.transform([1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn quarter_turn_around_z() {
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let pose = Pose {
            rotation: [half, 0.0, 0.0, half],
            translation: [10.0, 0.0, 0.0],
        };
        assert!(close(pose.transform([1.0, 0.0, 0.0]), [10.0, 1.0, 0.0]));
        assert!(close(pose.rotate([0.0, 1.0, 0.0]), [-1.0, 0.0, 0.0]));
    }

    #[test]
    fn bounds_follow_the_pose() {
        let half = std::f64::consts::FRAC_1_SQRT_2;
        let pose = Pose {
            rotation: [half, 0.0, 0.0, half],
            translation: [100.0, 0.0, 5.0],
        };
        let local = Bounds {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 1.0, 1.0],
        };
        let world = local.transformed(&pose);
        assert!(close(world.min, [99.0, 0.0, 5.0]));
        assert!(close(world.max, [100.0, 2.0, 6.0]));
    }

    #[test]
    fn empty_bounds() {
        let mut bounds = Bounds::EMPTY;
        assert!(bounds.is_empty());
        bounds.add([1.0, 2.0, 3.0]);
        assert!(!bounds.is_empty());
        assert_eq!(bounds.min, bounds.max);
    }
}
