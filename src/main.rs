use anyhow::Context;
use clap::Parser;
use e57_to_las::convert_file;
use e57_to_las::las_version::Version;
use e57_to_las::Result;

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

    let las_version = Version::try_from(args.las_version.as_str())?;

    convert_file(
        args.path,
        args.output,
        args.threads,
        args.stations,
        las_version,
    )
    .context("Failed to convert file")?;

    Ok(())
}
