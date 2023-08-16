mod extended_point;
mod utils;
extern crate rayon;
use anyhow::{Context, Result};
use clap::Parser;
use e57::E57Reader;
use extended_point::ExtendedPoint;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use las::Write;
use nalgebra::Point3;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    fs::File,
    io::{BufWriter, Write as IoWrite},
    path::Path,
    sync::Mutex,
};
use uuid::Uuid;

use crate::utils::*;

#[derive(Serialize)]
struct StationPoint {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,

    #[arg(short = 'P', long, default_value_t = false)]
    progress: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = args.path;
    let output_path = Path::new(&args.output);
    let has_progress = args.progress;

    let e57_reader = E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

    let pointclouds = e57_reader.pointclouds();

    let mut progress_bar = ProgressBar::hidden();

    let stations: Mutex<Vec<StationPoint>> = Mutex::new(Vec::new());

    if has_progress {
        let total_records: u64 = pointclouds.iter().map(|pc| pc.records).sum();
        progress_bar = ProgressBar::new(total_records);
        progress_bar.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg} ({eta})",
            )
            .unwrap()
            .with_key(
                "eta",
                |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                    write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
                },
            )
            .progress_chars("=>"),
        );
    }

    pointclouds
        .par_iter()
        .enumerate()
        .for_each(|(index, pointcloud)| -> () {
            let las_path = match construct_las_path(&output_path, index)
                .context("Couldn't create las path.")
            {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            };

            let transform = pointcloud
                .clone()
                .transform
                .unwrap_or(e57::Transform::default());
            let (rotation, translation) = get_rotations_and_translations(&transform);

            let mut builder = las::Builder::from((1, 4));
            builder.point_format.has_color = true;
            builder.generating_software = String::from("e57_to_las");
            builder.guid = match Uuid::parse_str(&pointcloud.guid.clone()).context("Invalid guid") {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            };

            let header = match builder.into_header() {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            };

            let writer = Mutex::new(match las::Writer::from_path(&las_path, header) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            });

            let mut e57_reader =
                match E57Reader::from_file(&input_path).context("Failed to open e57 file") {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error encountered: {}", e);
                        return ();
                    }
                };

            let mut pointcloud_reader = match e57_reader
                .pointcloud_simple(&pointcloud)
                .context("Unable to get point cloud iterator")
            {
                Ok(i) => i,
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            };
            pointcloud_reader.skip_invalid(true);

            println!("Saving pointcloud {} ...", index);

            let points: Vec<_> = pointcloud_reader.collect();
            let count = points.len() as f64;

            let sum = points.iter().fold((0.0, 0.0, 0.0), |acc, point| {
                let point = match point {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error encountered: {}", e);
                        return acc;
                    }
                };

                (
                    acc.0 + point.cartesian.x,
                    acc.1 + point.cartesian.y,
                    acc.2 + point.cartesian.z,
                )
            });

            stations.lock().unwrap().push(StationPoint {
                x: sum.0 / count,
                y: sum.1 / count,
                z: sum.2 / count,
            });

            points.par_iter().for_each(|p| {
                let point = match p {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Error encountered: {}", e);
                        return ();
                    }
                };

                let xyz = rotation.transform_point(&Point3::new(
                    point.cartesian.x,
                    point.cartesian.y,
                    point.cartesian.z,
                )) + translation;
                let las_rgb = ExtendedPoint::from(point.clone()).rgb_color;
                let las_intensity = get_intensity(point.intensity, point.intensity_invalid);

                let las_point = las::Point {
                    x: xyz.x,
                    y: xyz.y,
                    z: xyz.z,
                    intensity: las_intensity,
                    color: Some(las_rgb),
                    ..Default::default()
                };

                let mut writer_guard = writer.lock().unwrap();
                match writer_guard.write(las_point) {
                    Ok(_) => (),
                    Err(_e) => return,
                };

                progress_bar.inc(1);
            });

            let mut writer_guard = writer.lock().unwrap();
            match writer_guard.close() {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Error encountered: {}", e);
                    return ();
                }
            };
        });

    let stations_file = File::create(output_path.join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    progress_bar.finish_with_message("Finished convertion from e57 to las !");
    Ok(())
}
