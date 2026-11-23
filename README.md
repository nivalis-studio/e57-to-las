# E57 to LAS

[![Crates.io](https://img.shields.io/crates/v/e57-to-las.svg)](https://crates.io/crates/e57-to-las)
[![Documentation](https://docs.rs/e57-to-las/badge.svg)](https://docs.rs/e57-to-las)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-brightgreen.svg)](https://doc.rust-lang.org/nomicon/meet-safe-and-unsafe.html)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Convert E57 point clouds (laser scans) to LAS or LAZ: a Rust library and a command-line tool.
Built on the [`e57`](https://github.com/cry-inc/e57) and [`las`](https://github.com/gadomski/las-rs) crates.

- **Fast**: scans are decoded in parallel and points are encoded on the decoding threads; memory use does not grow with the file.
- **Complete**: coordinates with the scan pose applied, colors, intensity, GPS time, returns, scan index as point source ID, and optionally surface normals and scan grid row/column. The E57 coordinate system is copied as WKT.
- **Flexible output**: one merged file, one file per scan, or your own grouping; file names from a template; LAS or LAZ; files, memory or any writer.
- **Safe**: a file appears only once complete; a failed or cancelled conversion leaves nothing behind.
- **Embeddable**: progress events, cancellation, warnings returned instead of printed, typed errors, no `e57` or `las` types in the API.

## Command line

```sh
cargo install e57-to-las-cli
```

```sh
e57-to-las scan.e57                      # writes scan.las next to scan.e57
e57-to-las scan.e57 out/scan.laz         # LAZ, from the extension
e57-to-las scan.e57 --split              # one file per scan: scan/0.las, scan/1.las, ...
e57-to-las scan.e57 'out/{name}.las' --split --clouds 0,2-4
e57-to-las info scan.e57                 # scans, point counts, poses, attributes, CRS
```

The output path is a template: `{index}` (scan index), `{name}` (scan name), `{guid}`, `{key}` (output key) and `{stem}` (input file name) are replaced.
Two outputs resolving to the same path are refused before anything is written.

| Option | |
|---|---|
| `-s, --split` | One file per scan instead of one merged file |
| `-f, --format las\|laz` | Output format (default: from the extension) |
| `-L, --las-version 1.0..1.4` | LAS version (default 1.4; colors need 1.2+, a WKT CRS needs 1.4) |
| `--scale 0.001` | Coordinate precision in meters |
| `--crs from-e57\|none\|<WKT or file>` | Coordinate reference system |
| `--clouds 0,2,5-7` | Convert only these scans |
| `--stable-order` | Same point order on every run |
| `-T, --threads N` | Threads (default: all CPUs) |
| `--stations stations.json` | Write each scan's position and rotation |
| `--no-color`, `--no-intensity`, `--no-time`, `--no-returns`, `--no-source-id` | Drop an attribute |
| `--normals`, `--row-column` | Add normals / scan grid indices as LAS extra bytes |
| `--json`, `-q` | Machine-readable summary / no output |

Ctrl-C stops the conversion and removes unfinished files.

The layout of earlier versions (`--stations`) is reproduced with:

```sh
e57-to-las scan.e57 'out/las/{index}.las' --split --stations out/stations.json
```

## Library

```sh
cargo add e57-to-las
```

One call with the defaults:

```rust
let summary = e57_to_las::convert("scan.e57", "scan.las")?;
println!("{} points", summary.points_written());
```

Every option is on [`Converter`](https://docs.rs/e57-to-las/latest/e57_to_las/struct.Converter.html):

```rust
use e57_to_las::{CancelToken, Converter, Event, Grouping, LasVersion, Precision, output::FileOutput};

let cancel = CancelToken::new(); // clone it to a UI button, a signal handler, another task...

let summary = Converter::new()
    .las_version(LasVersion::V1_4)
    .precision(Precision::Scale(0.0005))
    .grouping(Grouping::PerCloud)
    .select(|cloud| cloud.point_count > 0)
    .map_points(|point| {
        point.z -= 47.3;                        // change points...
        point.intensity.is_none_or(|i| i > 500) // ...or drop them
    })
    .observer(|event| {
        if let Event::Progress { points_done, points_total } = event {
            eprintln!("{points_done}/{points_total}");
        }
    })
    .cancel_token(cancel.clone())
    .convert("scan.e57", FileOutput::new("out/{stem}_{index}.laz"))?;

for warning in &summary.warnings {
    eprintln!("warning: {warning}");
}
```

Inputs are paths, bytes in memory or any function returning a `Read + Seek`.
Outputs implement the [`Output`](https://docs.rs/e57-to-las/latest/e57_to_las/output/trait.Output.html) trait: `FileOutput` (templates, atomic writes), `MemoryOutput`, `output::from_fn(|target| ...)` for any `Write + Seek`, or your own type.
`Scan::open` reads the metadata only, to inspect scans before converting or to read points without writing LAS.

### Features

| Feature | Default | |
|---|---|---|
| `parallel` | yes | Decode scans on a thread pool ([rayon](https://github.com/rayon-rs/rayon)) |
| `crc32c` | yes | Hardware-accelerated E57 checksums |
| `laz` | | Write LAZ |
| `laz-parallel` | | Compress LAZ on several threads |
| `serde` | | `Serialize`/`Deserialize` for `CloudInfo`, `Summary`, options... |

## What gets written

- **Point format**: legacy LAS formats 0 to 3 (with or without GPS time and color), readable by every LAS tool. Normals (3 × `f32`) and row/column (2 × `i32`) are LAS extra bytes, described in an Extra Bytes VLR.
- **Precision**: 1 mm by default. The offset comes from the scan bounds in the E57 metadata, or from the first point. If points span more than about 2,147 km, a coarser scale is used and reported as a warning.
- **GPS time**: adjusted standard GPS time when the scan start time is known; otherwise seconds since the scan start, with a warning.
- **Coordinate system**: the E57 `coordinateMetadata` is written as a WKT VLR when it is WKT and the output is LAS 1.4.
- **Skipped points**: points without valid, finite coordinates are skipped and counted in the summary.
- **Point order**: scans sharing an output are read in parallel, so their points interleave. `--stable-order` (or `Converter::stable_order`) writes them in scan order.

## Development

The test fixtures are stored with Git LFS:

```sh
git lfs pull
cargo test --workspace --all-features
cargo bench -p e57-to-las --all-features   # needs fixtures/Trimble_StSulpice-Cloud-50mm.e57
```

The minimum supported Rust version is the latest stable release (`rust-version` in `Cargo.toml`).

## License

MIT. See [LICENSE](LICENSE).
