//! `e57-to-las`: convert E57 point clouds to LAS and LAZ from the command line.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use e57_to_las::output::{FileOutput, Format};
use e57_to_las::{
    Attributes, CancelToken, CloudInfo, Converter, Crs, Error, Event, Grouping, LasVersion,
    Precision, Scan, Summary,
};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

/// Convert E57 point clouds to LAS or LAZ.
///
/// Without a subcommand, converts: `e57-to-las scan.e57` writes `scan.las` next to the input.
#[derive(Debug, Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    convert: Option<ConvertArgs>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Convert an E57 file (the default command).
    Convert(ConvertArgs),
    /// Describe the point clouds of an E57 file.
    Info(InfoArgs),
}

#[derive(Debug, Args)]
struct ConvertArgs {
    /// The E57 file to convert.
    input: PathBuf,

    /// Output file, or a path template with {index}, {name}, {guid}, {key} or {stem}.
    ///
    /// Default: `<stem>.las` next to the input, or `<stem>/{index}.las` with --split.
    output: Option<PathBuf>,

    /// Write one file per point cloud (scan station) instead of merging them.
    #[arg(short, long)]
    split: bool,

    /// Output format. Default: from the output extension, LAS otherwise.
    #[arg(short, long, value_enum)]
    format: Option<FormatArg>,

    /// LAS version: 1.0 to 1.4. Colors need 1.2+, a WKT coordinate system 1.4.
    #[arg(short = 'L', long, default_value_t = LasVersion::V1_4)]
    las_version: LasVersion,

    /// Coordinate precision in meters.
    #[arg(long, default_value_t = 0.001)]
    scale: f64,

    /// Coordinate reference system: `from-e57`, `none`, a WKT string or a file holding WKT.
    #[arg(long, default_value = "from-e57")]
    crs: String,

    /// Convert only these point clouds, by index: `0,2,5-7`.
    #[arg(long, value_parser = parse_indices)]
    clouds: Option<IndexSet>,

    /// Write points in point cloud order, the same on every run.
    #[arg(long)]
    stable_order: bool,

    /// Number of threads. 0: one per CPU.
    #[arg(short = 'T', long, default_value_t = 0)]
    threads: usize,

    /// Write the pose of each converted point cloud (scan station) to this JSON file.
    #[arg(long, value_name = "PATH")]
    stations: Option<PathBuf>,

    /// Do not write colors.
    #[arg(long)]
    no_color: bool,
    /// Do not write intensities.
    #[arg(long)]
    no_intensity: bool,
    /// Do not write GPS times.
    #[arg(long)]
    no_time: bool,
    /// Do not write return numbers.
    #[arg(long)]
    no_returns: bool,
    /// Do not write the point cloud index as point source ID.
    #[arg(long)]
    no_source_id: bool,
    /// Write surface normals as extra bytes.
    #[arg(long)]
    normals: bool,
    /// Write scan grid row and column as extra bytes.
    #[arg(long)]
    row_column: bool,

    /// Print the summary as JSON on stdout.
    #[arg(long)]
    json: bool,

    /// No progress bar, no summary (errors and warnings are still printed).
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Debug, Args)]
struct InfoArgs {
    /// The E57 file.
    input: PathBuf,

    /// Print JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Las,
    Laz,
}

type IndexSet = std::collections::BTreeSet<usize>;

fn parse_indices(text: &str) -> Result<IndexSet, String> {
    let mut set = IndexSet::new();
    for part in text.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let number = |s: &str| {
            s.trim()
                .parse::<usize>()
                .map_err(|_| format!("{s:?} is not a point cloud index"))
        };
        match part.split_once('-') {
            Some((start, end)) => {
                let (start, end) = (number(start)?, number(end)?);
                if start > end {
                    return Err(format!("empty range {part}"));
                }
                set.extend(start..=end);
            }
            None => {
                set.insert(number(part)?);
            }
        }
    }
    if set.is_empty() {
        return Err("no point cloud index given".into());
    }
    Ok(set)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match (cli.command, cli.convert) {
        (Some(Command::Info(args)), _) => info(&args),
        (Some(Command::Convert(args)), _) | (None, Some(args)) => convert(args),
        (None, None) => {
            use clap::CommandFactory;
            let _ = Cli::command().print_help();
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if matches!(err.downcast_ref::<Error>(), Some(Error::Cancelled)) {
                eprintln!("cancelled");
            } else {
                eprintln!("error: {err}");
                for cause in err.chain().skip(1) {
                    eprintln!("  caused by: {cause}");
                }
            }
            ExitCode::FAILURE
        }
    }
}

fn default_output(input: &Path, split: bool, format: Option<FormatArg>) -> PathBuf {
    let stem = input
        .file_stem()
        .map_or_else(|| "output".into(), |s| s.to_string_lossy().into_owned());
    let extension = match format {
        Some(FormatArg::Laz) => "laz",
        _ => "las",
    };
    let dir = input.parent().unwrap_or(Path::new(""));
    if split {
        dir.join(stem).join(format!("{{index}}.{extension}"))
    } else {
        dir.join(format!("{stem}.{extension}"))
    }
}

fn crs(arg: &str) -> anyhow::Result<Crs> {
    Ok(match arg {
        "from-e57" => Crs::FromE57,
        "none" => Crs::None,
        text if Path::new(text).is_file() => Crs::Wkt(
            std::fs::read_to_string(text)
                .with_context(|| format!("cannot read the WKT file {text}"))?
                .trim()
                .to_owned(),
        ),
        wkt => Crs::Wkt(wkt.to_owned()),
    })
}

fn convert(args: ConvertArgs) -> anyhow::Result<()> {
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| default_output(&args.input, args.split, args.format));
    let file_output = FileOutput::new(&output);
    if args.split && !file_output.has_placeholders() {
        let stem = output.file_stem().unwrap_or_default().to_string_lossy();
        let extension = output
            .extension()
            .map_or("las".into(), |e| e.to_string_lossy());
        bail!(
            "with --split, the output must be a template such as {}",
            output
                .with_file_name(format!("{stem}_{{index}}.{extension}"))
                .display()
        );
    }

    let scan = Scan::open(&args.input)?;
    let attributes = Attributes::default()
        .color(!args.no_color)
        .intensity(!args.no_intensity)
        .time(!args.no_time)
        .returns(!args.no_returns)
        .point_source_id(!args.no_source_id)
        .normals(args.normals)
        .row_column(args.row_column);

    let token = CancelToken::new();
    let on_interrupt = token.clone();
    ctrlc::set_handler(move || on_interrupt.cancel()).context("cannot handle Ctrl-C")?;

    let bar = if args.quiet || args.json {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(0).with_style(
            ProgressStyle::with_template(
                "{spinner} [{elapsed_precise}] {wide_bar} {human_pos}/{human_len} points ({per_sec}, {eta})",
            )
            .unwrap_or_else(|_| ProgressStyle::default_bar()),
        )
    };
    let progress = bar.clone();

    let mut converter = Converter::new()
        .las_version(args.las_version)
        .precision(Precision::Scale(args.scale))
        .attributes(attributes)
        .crs(crs(&args.crs)?)
        .threads(args.threads)
        .stable_order(args.stable_order)
        .cancel_token(token)
        .observer(move |event| match event {
            Event::Started { points_total, .. } => progress.set_length(*points_total),
            Event::Progress {
                points_done,
                points_total,
            } => {
                progress.set_length(*points_total);
                progress.set_position(*points_done);
            }
            Event::Warning(warning) => progress.suspend(|| eprintln!("warning: {warning}")),
            _ => {}
        });
    if args.split {
        converter = converter.grouping(Grouping::PerCloud);
    }
    if let Some(format) = args.format {
        converter = converter.format(match format {
            FormatArg::Las => Format::Las,
            FormatArg::Laz => Format::Laz,
        });
    }
    if let Some(clouds) = args.clouds.clone() {
        let count = scan.clouds().len();
        if let Some(missing) = clouds.iter().find(|&&i| i >= count) {
            bail!("point cloud {missing} does not exist: the file has {count}");
        }
        converter = converter.select(move |cloud| clouds.contains(&cloud.index));
    }

    let result = converter.convert_scan(&scan, &file_output);
    bar.finish_and_clear();
    let summary = result?;

    if let Some(path) = &args.stations {
        write_stations(path, &scan, &summary)?;
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else if !args.quiet {
        report(&summary, args.stations.as_deref());
    }
    Ok(())
}

fn report(summary: &Summary, stations: Option<&Path>) {
    for output in &summary.outputs {
        let location = output
            .location
            .as_ref()
            .map_or_else(|| output.key.to_string(), |p| p.display().to_string());
        println!("{location}: {} points", output.points_written());
    }
    if let Some(path) = stations {
        println!("{}: stations", path.display());
    }
    let mut totals = format!(
        "{} points written in {} file(s)",
        summary.points_written(),
        summary.outputs.len()
    );
    if summary.points_skipped() > 0 {
        totals += &format!(
            ", {} skipped (invalid coordinates)",
            summary.points_skipped()
        );
    }
    println!("{totals}");
}

/// One entry of `stations.json`. `x`, `y` and `z` keep the layout of earlier versions.
#[derive(Serialize)]
struct Station<'a> {
    x: f64,
    y: f64,
    z: f64,
    /// Quaternion `[w, x, y, z]`.
    rotation: [f64; 4],
    name: Option<&'a str>,
    guid: Option<&'a str>,
}

fn write_stations(path: &Path, scan: &Scan, summary: &Summary) -> anyhow::Result<()> {
    let stations: BTreeMap<usize, Station<'_>> = summary
        .clouds()
        .filter_map(|c| scan.clouds().get(c.index))
        .map(|cloud: &CloudInfo| {
            let [x, y, z] = cloud.pose.translation;
            (
                cloud.index,
                Station {
                    x,
                    y,
                    z,
                    rotation: cloud.pose.rotation,
                    name: cloud.name.as_deref(),
                    guid: cloud.guid.as_deref(),
                },
            )
        })
        .collect();
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&stations)?;
    std::fs::write(path, json).with_context(|| format!("cannot write {}", path.display()))
}

#[derive(Serialize)]
struct InfoJson<'a> {
    guid: &'a str,
    coordinate_metadata: Option<&'a str>,
    point_count: u64,
    clouds: &'a [CloudInfo],
}

fn info(args: &InfoArgs) -> anyhow::Result<()> {
    let scan = Scan::open(&args.input)?;
    if args.json {
        let info = InfoJson {
            guid: scan.guid(),
            coordinate_metadata: scan.coordinate_metadata(),
            point_count: scan.point_count(),
            clouds: scan.clouds(),
        };
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("file:   {}", args.input.display());
    println!("guid:   {}", scan.guid());
    println!(
        "crs:    {}",
        scan.coordinate_metadata().map_or("(none)".into(), |crs| {
            let line: String = crs.chars().take(70).collect();
            if line.len() < crs.len() {
                format!("{line}…")
            } else {
                line
            }
        })
    );
    println!(
        "points: {} in {} point cloud(s)",
        scan.point_count(),
        scan.clouds().len()
    );
    for cloud in scan.clouds() {
        let mut attributes = Vec::new();
        for (on, name) in [
            (cloud.has_color, "color"),
            (cloud.has_intensity, "intensity"),
            (cloud.has_time, "time"),
            (cloud.has_returns, "returns"),
            (cloud.has_normals, "normals"),
            (cloud.has_row_column, "row/column"),
        ] {
            if on {
                attributes.push(name);
            }
        }
        let [x, y, z] = cloud.pose.translation;
        println!();
        println!(
            "[{}] {}",
            cloud.index,
            cloud.name.as_deref().unwrap_or("(unnamed)")
        );
        println!("    points:     {}", cloud.point_count);
        println!("    position:   {x:.3}, {y:.3}, {z:.3}");
        if let Some(bounds) = cloud.bounds {
            let [a, b, c] = bounds.min;
            let [d, e, f] = bounds.max;
            println!("    bounds:     ({a:.3}, {b:.3}, {c:.3}) to ({d:.3}, {e:.3}, {f:.3})");
        }
        println!(
            "    attributes: {}",
            if attributes.is_empty() {
                "-".into()
            } else {
                attributes.join(", ")
            }
        );
        if let Some(model) = cloud.sensor_model.as_deref() {
            println!("    scanner:    {model}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_sets() {
        assert_eq!(
            parse_indices("0,2,5-7").unwrap(),
            IndexSet::from([0, 2, 5, 6, 7])
        );
        assert!(parse_indices("3-1").is_err());
        assert!(parse_indices("a").is_err());
        assert!(parse_indices("").is_err());
    }

    #[test]
    fn default_outputs() {
        let input = Path::new("data/scan.e57");
        assert_eq!(
            default_output(input, false, None),
            PathBuf::from("data/scan.las")
        );
        assert_eq!(
            default_output(input, false, Some(FormatArg::Laz)),
            PathBuf::from("data/scan.laz")
        );
        assert_eq!(
            default_output(input, true, None),
            PathBuf::from("data/scan/{index}.las")
        );
    }

    #[test]
    fn parses_the_default_command_and_subcommands() {
        let cli = Cli::try_parse_from(["e57-to-las", "in.e57", "out.las", "--split", "-L", "1.2"])
            .unwrap();
        let args = cli.convert.unwrap();
        assert_eq!(args.input, PathBuf::from("in.e57"));
        assert_eq!(args.output, Some(PathBuf::from("out.las")));
        assert!(args.split);
        assert_eq!(args.las_version, LasVersion::V1_2);

        let cli = Cli::try_parse_from(["e57-to-las", "info", "in.e57", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Info(InfoArgs { json: true, .. }))
        ));

        assert!(Cli::try_parse_from(["e57-to-las", "in.e57", "-L", "1.5"]).is_err());
    }
}
