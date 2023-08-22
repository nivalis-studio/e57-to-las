use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,

    #[arg(short = 'P', long, default_value_t = false)]
    progress: bool,

    #[arg(short = 'D', long, default_value_t = false)]
    debug: bool,

    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,
}

pub(crate) fn construct_las_path(output_path: &String, index: usize) -> Result<PathBuf> {
    let output_sub_dir_path = Path::new(&output_path).join("las");

    std::fs::create_dir_all(&output_sub_dir_path).context(format!(
        "Couldn't find or create output dir {}.",
        output_sub_dir_path
            .to_str()
            .context("Invalid output dir path encoding.")?
    ))?;

    let las_path = output_sub_dir_path.join(format!("{}{}", index, ".las"));

    Ok(las_path)
}
