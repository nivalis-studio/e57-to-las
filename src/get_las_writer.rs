use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::{Context, Result};
use las::Vector;
use uuid::Uuid;

fn find_smallest_scale(x: f64) -> f64 {
    let mut scale = 0.001;
    let min_i32 = i32::MIN as f64;
    let max_i32 = i32::MAX as f64;

    while (x / scale).round() < min_i32 || (x / scale).round() > max_i32 {
        scale += 0.0001;
    }

    scale
}

pub(crate) fn get_las_writer(
    guid: impl Into<String>,
    output_path: PathBuf,
    max_cartesian: f64,
) -> Result<las::Writer<BufWriter<File>>> {
    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    let offset = 0.0;

    let scale = find_smallest_scale(max_cartesian);

    let transform = las::Transform { scale, offset };
    builder.transforms = Vector {
        x: transform,
        y: transform,
        z: transform,
    };
    builder.guid = Uuid::parse_str(&guid.into().replace("_", "-")).unwrap_or(Uuid::new_v4());

    let header = builder.into_header().context("Error encountered: ")?;

    let writer = las::Writer::from_path(&output_path, header).context("Error encountered: ")?;

    Ok(writer)
}
