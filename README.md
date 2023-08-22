# E57 to LAS conversion

[![Crates.io](https://img.shields.io/crates/v/e57_to_las.svg)](https://crates.io/crates/e57_to_las)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Dependencies](https://deps.rs/repo/github/wildweb-io/e57_to_las/status.svg)](https://deps.rs/repo/github/wildweb-io/e57_to_las)

A utility to convert E57 point cloud files to LAS format. It is based on the [E57 crate](https://github.com/cry-inc/e57) and the [LAS crate](https://github.com/gadomski/las-rs).

## About

`e57_to_las` is an open-source tool designed to convert point cloud files in the E57 format to the LAS format. The conversion retains all the relevant point data and associated metadata, making it easier for users to work with point cloud data in environments that primarily support LAS. We use is to convert E57 files to LAS for use in [Potree](https://github.com/potree/potree/) for web-based point cloud visualization.

## Features

- [x] Parallel processing using `rayon` for faster conversion.
- [ ] Progress bar support to track the conversion process.
- [x] Error handling to ensure corrupted or unsupported files do not halt the process.
- [x] Generates a JSON file (`stations.json`) containing station points after conversion.

## Usage

```bash
e57_to_las [OPTIONS]
```

### Options

- `-p, --path <path>`: The path to the input E57 file.
- `-o, --output <output>`: The output directory for the converted LAS files (default: `./`).
- `-P, --progress`: Display a progress bar (default: off).
- `-T, --threads <threads>`: Number of threads for parallel processing (default: 0 = max possible).

## How It Works

1. Reads the provided E57 file.
2. Initializes the progress bar if the `-P` option is provided.
3. Loops through the point clouds in the E57 file in parallel.
4. For each point cloud, it writes the points to a corresponding LAS file.
5. After processing all point clouds, it calculates and writes the station points to `stations.json` in the output directory.

## Contribution

If you'd like to contribute to the development of this tool, please create an issue or pull request on our GitHub repository. All contributions are welcome!

## Dependencies

Here are some of the main dependencies used:

- `rayon`: Parallelism
- `clap`: Command-line argument parsing
- `e57`: E57 file format reader
- `las`: LAS file format writer
- `indicatif`: Progress bar
- `uuid`: For GUID processing
- `serde`: For serialization and deserialization of data

## License

Open-source MIT. See [LICENSE](LICENSE) for details.
