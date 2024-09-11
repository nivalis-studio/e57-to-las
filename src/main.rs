use anyhow::Context;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use clap::Parser;
use e57_to_las::{
    extensions::{
        e57::{point_cloud::E57PointCloudSimpleExt, reader::E57ReaderExt},
        las::{point::LasPointsExt, version::LasVersionExt},
    },
    Result,
};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,

    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,

    #[arg(short = 'S', long, default_value_t = false)]
    stations: bool,

    #[arg(short = 'L', long)]
    las_version: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .context("Failed to initialize the global thread pool")?;

    if args.stations {
        convert_file_stations(args)
    } else {
        convert_file_simple(args)
    }
}

fn convert_file_simple(args: Args) -> Result<()> {
    let mut e57_reader =
        e57::E57Reader::from_file(&args.path).context("Failed to open e57 file")?;

    let las_points = e57_reader.to_las_points();

    let mut header_builder = las_points.default_builder();

    if let Ok(uuid) = Uuid::parse_str(&e57_reader.guid().replace('_', "-")) {
        header_builder.guid = uuid
    };

    if let Some(version) = args.las_version {
        header_builder.version = las::Version::try_from_str(&version)?;
    }

    let header = header_builder
        .into_header()
        .context("Failed to build las header")?;

    let output_path = create_path(Path::new(&args.output).join("result.las"))?;

    let mut writer =
        las::Writer::from_path(output_path, header).context("Faild to get las writer")?;

    las_points
        .into_iter()
        .try_for_each(|p| writer.write_point(p).context("Failed to write point"))?;

    writer.close().context("Failed to close las writer")?;

    Ok(())
}

fn convert_file_stations(args: Args) -> Result<()> {
    let e57_reader = e57::E57Reader::from_file(&args.path).context("Failed to open e57 file")?;
    let pointclouds = e57_reader.pointclouds();
    let reader_mutex = Mutex::new(e57_reader);

    let output_path = create_path(Path::new(&args.output).join("results"))?;

    pointclouds
        .par_iter()
        .enumerate()
        .try_for_each(|(i, pc)| -> Result<()> {
            let mut reader = reader_mutex.lock().unwrap();
            let pointcloud_simple = reader.pointcloud_simple(pc).unwrap();

            let las_points = pointcloud_simple.to_las_points();

            let mut header_builder = las_points.default_builder();

            if let Ok(uuid) = Uuid::parse_str(&reader.guid().replace('_', "-")) {
                header_builder.guid = uuid
            };

            if let Some(version) = &args.las_version {
                header_builder.version = las::Version::try_from_str(version)?;
            }

            let header = header_builder
                .into_header()
                .context("Failed to build las header")?;

            let mut writer = las::Writer::from_path(output_path.join(format!("{}.las", i)), header)
                .context("Faild to get las writer")?;

            las_points
                .into_iter()
                .try_for_each(|p| writer.write_point(p).context("Failed to write point"))?;

            writer.close().context("Failed to close las writer")?;
            Ok(())
        })?;

    let stations_map: HashMap<usize, Value> = pointclouds
        .iter()
        .enumerate()
        .map(|(i, pc)| {
            let transform = pc.transform.clone().unwrap_or_default().translation;

            let coordinates = json!({
                            "x": transform.x,
                            "y": transform.y,
                            "z": transform.z
            });

            (i, coordinates)
        })
        .collect();

    let stations_file =
        File::create(output_path.join("stations.json")).context("Couldn't create stations path")?;

    let mut writer = BufWriter::new(stations_file);

    serde_json::to_writer(&mut writer, &stations_map).context("Couldn't write stations file")?;

    writer.flush().context("Couldn't flush stations writer")?;

    Ok(())
}

fn create_path(path: PathBuf) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Invalid path.",
        ))
        .context("Invalid path")?;

    std::fs::create_dir_all(parent).context(format!(
        "Couldn't find or create output dir {}.",
        parent
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    Ok(path)
}
