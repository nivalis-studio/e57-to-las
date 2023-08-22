extern crate rayon;
use anyhow::{Context, Result};
use clap::Parser;
use e57::E57Reader;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write as IoWrite},
    path::Path,
    sync::Mutex,
};

use e57_to_las::pc_converter::point_cloud_converter;
use e57_to_las::stations::StationPoint;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,

    #[arg(short = 'P', long, default_value_t = false)]
    progress: bool,

    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let input_path = args.path;
    let output_path = Path::new(&args.output);
    let has_progress = args.progress;
    let number_of_threads = args.threads;

    rayon::ThreadPoolBuilder::new()
        .num_threads(number_of_threads)
        .build_global()
        .context("Failed to initialize the global thread pool")?;

    let e57_reader = E57Reader::from_file(&input_path).context("Failed to open e57 file")?;

    let pointclouds = e57_reader.pointclouds();

    let mut progress_bar = ProgressBar::hidden();

    let stations: Mutex<Vec<StationPoint>> = Mutex::new(Vec::new());

    if has_progress {
        let total_records: u64 = pointclouds.iter().map(|pc| pc.records).sum();
        let progress_style = ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {msg} ({eta})",
        )
        .context("Error setting up progress bar template")?
        .with_key(
            "eta",
            |state: &ProgressState, w: &mut dyn std::fmt::Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            },
        )
        .progress_chars("=>");

        progress_bar = ProgressBar::new(total_records);
        progress_bar.set_style(progress_style);
    }

    pointclouds
        .par_iter()
        .enumerate()
        .for_each(|(index, pointcloud)| -> () {
            let mut converter_result =
                match point_cloud_converter(index, pointcloud, &input_path, output_path) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error encountered: {}", e);
                        return;
                    }
                };

            stations.lock().unwrap().append(&mut converter_result);
        });

    let stations_file = File::create(output_path.join("stations.json"))?;
    let mut writer = BufWriter::new(stations_file);
    serde_json::to_writer(&mut writer, &stations)?;
    writer.flush()?;

    progress_bar.finish_with_message("Finished conversion from e57 to las !");
    Ok(())
}
