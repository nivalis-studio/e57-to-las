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
- [x] Errors are reported with context and stop the conversion. A failed conversion leaves no LAS file behind. Points with invalid coordinates are skipped, and the number skipped per point cloud is printed.
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

### Library

To use this as a crate in your own project, add it to your `Cargo.toml`:

```bash
cargo add e57-to-las
```

You can then use it in your code as follows:

```rust
use e57_to_las::{convert_file, LasVersion};

fn main() {
    let input_path = String::from("path/to/input.e57");
    let output_path = String::from("path/to/output/directory");
    let number_of_threads = 0; // 0 = max possible
    let as_stations = true;
    let las_version = LasVersion::new(1, 4).expect("Failed to create LAS version"); // 1.0 to 1.4
    convert_file(input_path, output_path, number_of_threads, as_stations, las_version)
        .expect("conversion failed");
}
```

## Contribution

If you'd like to contribute to the development of this tool, please create an issue or pull request on our GitHub repository. All contributions are welcome!

## Dependencies

Here are some of the main dependencies used:

- `rayon`: Parallelism
- `clap`: Command-line argument parsing
- `e57`: E57 file format reader
- `las`: LAS file format writer
- `uuid`: For GUID processing
- `serde`: For serialization and deserialization of data

## License

Open-source MIT. See [LICENSE](LICENSE) for details.
