# E57 to LAS conversion

[![Crates.io](https://img.shields.io/crates/v/e57-to-las.svg)](https://crates.io/crates/e57-to-las)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![Documentation](https://docs.rs/e57-to-las/badge.svg)](https://docs.rs/e57-to-las)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Dependencies](https://deps.rs/repo/github/nivalis-studio/e57-to-las/status.svg)](https://deps.rs/repo/github/nivalis-studio/e57-to-las)

A utility to convert E57 point cloud files to LAS format. It is based on the [E57 crate](https://github.com/cry-inc/e57) and the [LAS crate](https://github.com/gadomski/las-rs).

## About

`e57-to-las` is an open-source tool designed to convert point cloud files in the E57 format to the LAS format. The conversion retains all the relevant point data and associated metadata, making it easier for users to work with point cloud data in environments that primarily support LAS. We use is to convert E57 files to LAS for use in [Potree](https://github.com/potree/potree/) for web-based point cloud visualization.

## Features

- [x] Parallel processing using `rayon` for faster conversion.
- [x] Error handling to ensure corrupted or unsupported files do not halt the process.
- [x] Optional pointclouds splitting in distinct LAS files and generation of station file (`stations.json`), containing spatial coordinates of station points. This is activated by adding the `--stations` flag, and the station points are calculated based on the transformation translations of the point clouds.

## Usage

```bash
e57_to_las [OPTIONS]
```

To use this as a crate in your own project, add the following to your `Cargo.toml`:

```toml
[dependencies]
e57-to-las = "0.6.1"
```

You can then use it in your code as follows:

```rust
use e57_to_las::convert_file;

fn main() {
    let input_path = String::from("path/to/input.e57");
    let output_path = String::from("path/to/output/directory");
    let number_of_threads = 0; // 0 = max possible
    let as_stations = true;
    let las_version = (1, 4); // 1.0 to 1.4
    convert_file(input_path, output_path, number_of_threads, as_stations, las_version);
}
```

### Options

- `-p, --path <path>`: The path to the input E57 file.
- `-o, --output <output>`: The output directory for the converted LAS files (default: `./`).
- `-T, --threads <threads>`: Number of threads for parallel processing (default: 0 = max possible).
- `-S, --stations <stations>`: Whether to convert e57 file in distinct stations (default: false).
- `-L, --las_version <las_version>`: Version of LAS format used for output file. Default one is (1, 4). Currently possible: (1, 0) to (1, 4).

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
