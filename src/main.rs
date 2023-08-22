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

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .context("Failed to initialize the global thread pool")?;

    convert_file(args.path, args.output).context("Failed to convert file")?;
    Ok(())
}
