extern crate rayon;
use anyhow::{Context, Result};
use clap::Parser;

use e57_to_las::convert_file;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    path: String,

    #[arg(short, long, default_value_t = String::from("./"))]
    output: String,

    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    convert_file(args.path, args.output, args.threads).context("Failed to convert file")?;
    Ok(())
}
