use std::{fs::File, io::BufWriter};

use crate::utils::construct_las_path;
use anyhow::{Context, Result};
use e57::PointCloud;
use uuid::Uuid;

pub(crate) fn get_las_writer(
    index: usize,
    pointcloud: &PointCloud,
    output_path: &String,
) -> Result<las::Writer<BufWriter<File>>> {
    let las_path =
        construct_las_path(output_path, index).context("Unable to create file path: ")?;

    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    builder.guid =
        Uuid::parse_str(&pointcloud.guid.clone().replace("_", "-")).unwrap_or(Uuid::new_v4());

    let header = builder.into_header().context("Error encountered: ")?;

    let writer = las::Writer::from_path(&las_path, header).context("Error encountered: ")?;

    Ok(writer)
}
