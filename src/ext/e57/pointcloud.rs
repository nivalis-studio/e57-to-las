pub trait E57PointCloudExt {
    fn global_bounds(&self) -> Option<([f64; 3], [f64; 3])>;
}

impl E57PointCloudExt for e57::PointCloud {
    fn global_bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let b = self.get_cartesian_bounds()?;
        let local_min = [b.x_min?, b.y_min?, b.z_min?];
        let local_max = [b.x_max?, b.y_max?, b.z_max?];

        let Some(t) = &self.transform else {
            return Some((local_min, local_max));
        };

        let corners = [
            [local_min[0], local_min[1], local_min[2]],
            [local_min[0], local_min[1], local_max[2]],
            [local_min[0], local_max[1], local_min[2]],
            [local_min[0], local_max[1], local_max[2]],
            [local_max[0], local_min[1], local_min[2]],
            [local_max[0], local_min[1], local_max[2]],
            [local_max[0], local_max[1], local_min[2]],
            [local_max[0], local_max[1], local_max[2]],
        ];

        let (mut w, mut x, mut y, mut z) = (t.rotation.w, t.rotation.x, t.rotation.y, t.rotation.z);
        let n = (w * w + x * x + y * y + z * z).sqrt();
        if n > 0.0 {
            w /= n;
            x /= n;
            y /= n;
            z /= n;
        }

        let r = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ];
        let tv = [t.translation.x, t.translation.y, t.translation.z];

        let mut global_min = [f64::INFINITY; 3];
        let mut global_max = [f64::NEG_INFINITY; 3];

        for corner in corners {
            let transformed = [
                r[0][0] * corner[0] + r[0][1] * corner[1] + r[0][2] * corner[2] + tv[0],
                r[1][0] * corner[0] + r[1][1] * corner[1] + r[1][2] * corner[2] + tv[1],
                r[2][0] * corner[0] + r[2][1] * corner[1] + r[2][2] * corner[2] + tv[2],
            ];

            for k in 0..3 {
                global_min[k] = global_min[k].min(transformed[k]);
                global_max[k] = global_max[k].max(transformed[k]);
            }
        }

        Some((global_min, global_max))
    }
}
