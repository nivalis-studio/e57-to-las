# E57 to LAS conversion

[![Crates.io](https://img.shields.io/crates/v/e57-to-las.svg)](https://crates.io/crates/e57-to-las)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![Documentation](https://docs.rs/e57-to-las/badge.svg)](https://docs.rs/e57-to-las)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Dependencies](https://deps.rs/repo/github/nivalis-studio/e57-to-las/status.svg)](https://deps.rs/repo/github/nivalis-studio/e57-to-las)

A utility to convert E57 point cloud files to LAS format. It is based on the [E57 crate](https://github.com/cry-inc/e57) and the [LAS crate](https://github.com/gadomski/las-rs).

## About

`e57-to-las` is an open-source tool designed to convert point cloud files in the E57 format to the LAS format. The conversion retains all the relevant point data and associated metadata, making it easier for users to work with point cloud data in environments that primarily support LAS. We use it to convert E57 files to LAS for use in [Potree](https://github.com/potree/potree/) for web-based point cloud visualization.

## Features

- [x] Parallel processing using `rayon` for faster conversion.
- [x] Streaming conversion: points are written as they are decoded, so memory use stays roughly constant regardless of the number of points.
- [x] Errors are reported with context and stop the conversion. Points with invalid coordinates are skipped. The command-line tool prints how many were skipped; library callers read the count from `Summary`.
- [x] Optional pointclouds splitting in distinct LAS files and generation of station file (`stations.json`), containing spatial coordinates of station points. This is activated by adding the `--stations` flag, and the station points are calculated based on the transformation translations of the point clouds.

## Usage

Install the command-line tool with `cargo install e57-to-las`, then run:

```bash
e57-to-las [OPTIONS] --path <PATH>
```

To get changes that are not yet released on crates.io, install from the repository instead: `cargo install --git https://github.com/nivalis-studio/e57-to-las`.

For example:

```bash
e57-to-las --path scan.e57 --output out/ --stations --las-version 1.2
```

### Options

- `-p, --path <PATH>`: The path to the input E57 file.
- `-o, --output <OUTPUT>`: The output directory for the converted LAS files (default: `./`).
- `-T, --threads <THREADS>`: Number of threads for parallel processing (default: 0 = max possible).
- `-S, --stations`: Write each point cloud (station) to its own LAS file and generate `stations.json`. Off by default.
- `-L, --las-version <LAS_VERSION>`: Version of LAS format used for the output files, from `1.0` to `1.4` (default: `1.4`). LAS 1.0 and 1.1 cannot store colors: converting a colored scan with these versions fails with an error, so use `1.2` or newer.
- `-h, --help`: Print help.
- `-V, --version`: Print version.

### Output

- Without `--stations`, all point clouds are merged into `<output>/las/0.las`. The point clouds are decoded in parallel, so their points can be interleaved in the file and the order can differ between runs; the set of points is always the same. Use `--threads 1` to keep the points in point cloud order.
- With `--stations`, each point cloud is written to `<output>/las/<index>.las`, and `<output>/stations.json` maps each index to the `x`/`y`/`z` translation of that scan.
- Coordinates are stored with 1 mm precision. A coarser precision is only used when the points span more than about 2,147 km.
- The tool prints how many points were written, the path of each file it wrote, and, when some points had invalid coordinates, how many were skipped.

### Library

To use this as a crate in your own project, add it to your `Cargo.toml`:

```bash
cargo add e57-to-las
```

You can then use it in your code as follows:

```rust
use e57_to_las::{LasVersion, Options, convert_file};

fn main() -> e57_to_las::Result<()> {
    let options = Options {
        las_version: LasVersion::new(1, 4)?, // 1.0 to 1.4
        threads: 0, // 0 = one worker per logical CPU
        stations: true,
    };
    let summary = convert_file("path/to/input.e57", "path/to/output/directory", &options)?;
    println!(
        "Wrote {} points ({} skipped)",
        summary.points_written, summary.points_skipped
    );
    Ok(())
}
```

The `cli` feature (on by default) builds the `e57-to-las` binary and pulls in `clap`. Disable default features to depend on the library without it:

```toml
e57-to-las = { version = "0.9.0", default-features = false }
```

## Contribution

If you'd like to contribute to the development of this tool, please create an issue or pull request on our GitHub repository. All contributions are welcome!

## Dependencies

Here are some of the main dependencies used:

- `rayon`: Parallelism
- `clap`: Command-line argument parsing (optional `cli` feature, enabled by default)
- `e57`: E57 file format reader
- `las`: LAS file format writer
- `uuid`: For GUID processing
- `serde`: For serialization and deserialization of data

## License

Open-source MIT. See [LICENSE](LICENSE) for details.
