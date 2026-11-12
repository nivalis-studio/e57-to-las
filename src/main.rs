use anyhow::Context;
use clap::Parser;
use e57_to_las::{LasVersion, Options, Summary, convert};

#[derive(Debug, Parser)]
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

    #[arg(short = 'L', long, default_value_t = LasVersion::default())]
    las_version: LasVersion,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let summary = convert(
        args.path,
        args.output,
        &Options {
            las_version: args.las_version,
            threads: args.threads,
            stations: args.stations,
        },
    )
    .context("Failed to convert file")?;

    report(&summary);
    Ok(())
}

/// Prints the files [`convert`] wrote and how many points were skipped.
fn report(summary: &Summary) {
    println!("Wrote {} points", summary.points_written);
    for path in &summary.files {
        println!("{}", path.display());
    }
    if summary.points_skipped > 0 {
        println!(
            "Skipped {} points with invalid coordinates",
            summary.points_skipped
        );
    }
    if summary.coarse_scale {
        println!("Coordinates span more than 2147 km: wrote with a scale coarser than 1 mm");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_las_version_and_defaults_to_1_4() {
        let args = Args::try_parse_from(["e57-to-las", "--path", "in.e57"]).expect("parse");
        assert_eq!(args.las_version, LasVersion::new(1, 4).expect("version"));
        assert!(!args.stations);
        assert_eq!(args.threads, 0);
    }

    #[test]
    fn parses_explicit_las_version() {
        let args = Args::try_parse_from([
            "e57-to-las",
            "--path",
            "in.e57",
            "--las-version",
            "1.2",
            "--stations",
            "--threads",
            "3",
        ])
        .expect("parse");
        assert_eq!(args.las_version, LasVersion::new(1, 2).expect("version"));
        assert!(args.stations);
        assert_eq!(args.threads, 3);
    }

    #[test]
    fn rejects_an_unknown_las_version() {
        let err = Args::try_parse_from(["e57-to-las", "--path", "in.e57", "--las-version", "1.5"])
            .expect_err("1.5 is not a LAS version");
        let message = err.to_string();
        assert!(
            message.contains("Invalid LAS version"),
            "unexpected parse error: {message}"
        );
    }
}
