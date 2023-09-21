use std::{fs::File, io::BufWriter, path::PathBuf};

use anyhow::{Context, Result};
use uuid::Uuid;

pub(crate) fn get_las_writer(
    guid: impl Into<String>,
    output_path: PathBuf,
) -> Result<las::Writer<BufWriter<File>>> {
    let mut builder = las::Builder::from((1, 4));
    builder.point_format.has_color = true;
    builder.generating_software = String::from("e57_to_las");
    builder.guid = Uuid::parse_str(&guid.into().replace("_", "-")).unwrap_or(Uuid::new_v4());

    let header = builder.into_header().context("Error encountered: ")?;

    let writer = las::Writer::from_path(&output_path, header).context("Error encountered: ")?;

    Ok(writer)
}
