use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use clap::Parser;
use e57_to_las::{ConvertOptions, Event, EventCallback, LasVersion, Result, parallel};
use serde::Serialize;

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

    #[arg(short = 'L', long, default_value_t = String::from("1.4"))]
    las_version: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let las_version = LasVersion::from_str(&args.las_version)?;

    let pointclouds = Arc::new(Mutex::new(vec![]));

    let pcs = Arc::clone(&pointclouds);
    let on_event: Option<EventCallback> = args.stations.then_some(Arc::new(move |e| match e {
        Event::PointCloudStarted {
            idx, translation, ..
        } => {
            println!("Saving pointcloud {idx}...");

            let mut pointclouds = pcs.lock().unwrap();

            let (x, y, z) = translation;
            let station_point = StationPoint { x, y, z };

            pointclouds.push((idx, station_point));
        }
        Event::PointCloudEnded { idx } => {
            println!("Saved pointcloud {idx}");
        }
        _ => {}
    }));

    let mut opts = ConvertOptions {
        las_version,
        on_event,
        ..Default::default()
    };

    if args.threads != 0 {
        opts.workers = args.threads;
    }

    let input_path = args.path;
    let output_path = PathBuf::from(args.output);

    if args.stations {
        parallel::convert_split(input_path, output_path.clone(), &opts).unwrap();
    } else {
        parallel::convert(&input_path, output_path.clone(), &opts).unwrap();
    }

    let stations: BTreeMap<usize, StationPoint> = {
        let mut guard = pointclouds.lock().unwrap();

        let vec = std::mem::take(&mut *guard);

        vec.into_iter().collect()
    };

    let stations_file = File::create(output_path.parent().unwrap().join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations).unwrap();
    writer.flush()?;

    Ok(())
}

#[derive(Debug, Serialize)]
struct StationPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
