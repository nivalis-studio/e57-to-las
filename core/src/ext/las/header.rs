use crate::{
    ConvertOptions, Result,
    ext::e57::E57PointCloudExt,
    ext::las::{LasFormatAttributes, LasFormatExt},
};

pub(crate) trait LasHeaderExt: Sized {
    fn from_pointclouds(
        pointclouds: &[e57::PointCloud],
        opts: &ConvertOptions,
    ) -> Result<(Self, las::point::Format)>;

    fn from_pointcloud(
        pointcloud: &e57::PointCloud,
        opts: &ConvertOptions,
    ) -> Result<(Self, las::point::Format)>;
}

impl LasHeaderExt for las::Header {
    fn from_pointclouds(
        pointclouds: &[e57::PointCloud],
        opts: &ConvertOptions,
    ) -> Result<(Self, las::point::Format)> {
        let mut has_color = false;
        let mut has_time = false;
        let mut has_bounds = false;
        let mut global_min = [f64::INFINITY; 3];
        let mut global_max = [f64::NEG_INFINITY; 3];

        for pc in pointclouds {
            has_color |= pc.has_color();
            has_time |= pc.has_timestamp();
            if let Some((min, max)) = pc.global_bounds() {
                has_bounds = true;
                for k in 0..3 {
                    global_min[k] = global_min[k].min(min[k]);
                    global_max[k] = global_max[k].max(max[k]);
                }
            }
        }

        let point_format = las::point::Format::from_attributes(LasFormatAttributes {
            has_color,
            has_time,
            las_version: opts.las_version,
        });

        let combined_bounds = has_bounds.then_some((global_min, global_max));

        let mut builder = preset_header(point_format, combined_bounds, opts)?;

        if let Some(hook_fn) = &opts.header_hook {
            hook_fn(&mut builder);
        }

        let header = builder.into_header()?;
        let point_format = *header.point_format();

        Ok((header, point_format))
    }

    fn from_pointcloud(
        pointcloud: &e57::PointCloud,
        opts: &ConvertOptions,
    ) -> Result<(Self, las::point::Format)> {
        let has_color = pointcloud.has_color();
        let has_time = pointcloud.has_timestamp();
        let bounds = pointcloud.global_bounds();

        let point_format = las::point::Format::from_attributes(LasFormatAttributes {
            has_color,
            has_time,
            las_version: opts.las_version,
        });

        let mut builder = preset_header(point_format, bounds, opts)?;

        if let Some(hook_fn) = &opts.header_hook {
            hook_fn(&mut builder);
        }

        let header = builder.into_header()?;
        let point_format = *header.point_format();

        Ok((header, point_format))
    }
}

fn preset_header(
    point_format: las::point::Format,
    bounds: Option<([f64; 3], [f64; 3])>,
    opts: &ConvertOptions,
) -> Result<las::Builder> {
    let mut b = las::Builder::from(las::Version::from(opts.las_version));

    b.point_format = point_format;
    b.generating_software = String::from("e57_to_las");

    let mut scale = opts.scale.value();
    let mut offset = [0.0; 3];

    if let Some((global_min, global_max)) = bounds {
        offset = [
            (global_min[0] + global_max[0]) * 0.5,
            (global_min[1] + global_max[1]) * 0.5,
            (global_min[2] + global_max[2]) * 0.5,
        ];

        let max_extent_from_center = (0..3)
            .map(|i| (global_max[i] - global_min[i]) * 0.5)
            .fold(0.0, f64::max);
        scale = opts.scale.safe_value(max_extent_from_center);
    }

    b.transforms.x = las::Transform {
        offset: offset[0],
        scale,
    };
    b.transforms.y = las::Transform {
        offset: offset[1],
        scale,
    };
    b.transforms.z = las::Transform {
        offset: offset[2],
        scale,
    };

    Ok(b)
}
