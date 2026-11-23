//! Conversion benchmarks on the repository fixtures (fetch them with `git lfs pull`).
//!
//! Outputs go to memory, so the numbers measure decoding, encoding and
//! compression, not the disk.
//!
//! ```sh
//! cargo bench -p e57-to-las --all-features
//! ```

#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used, clippy::print_stderr)]

use std::path::PathBuf;

use divan::Bencher;
use e57_to_las::output::{Format, MemoryOutput};
use e57_to_las::{Converter, Grouping, Scan};

fn main() {
    divan::main();
}

fn fixture(name: &str) -> Option<Scan> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name);
    let bytes = std::fs::read(&path).ok()?;
    if bytes.starts_with(b"version https://git-lfs") {
        return None;
    }
    // Read once into memory so file caching does not skew the first sample.
    Some(Scan::open(bytes).expect("valid fixture"))
}

const TRIMBLE: &str = "Trimble_StSulpice-Cloud-50mm.e57";

fn run(bencher: Bencher, converter: Converter) {
    let Some(scan) = fixture(TRIMBLE) else {
        eprintln!("skipped: {TRIMBLE} is missing, run `git lfs pull`");
        return;
    };
    bencher
        .counter(divan::counter::ItemsCount::new(scan.point_count()))
        .bench(|| converter.convert_scan(&scan, MemoryOutput::new()).unwrap());
}

#[divan::bench(args = [1, 0], sample_count = 10)]
fn las_merged(bencher: Bencher, threads: usize) {
    run(bencher, Converter::new().threads(threads));
}

#[divan::bench(sample_count = 10)]
fn las_per_cloud(bencher: Bencher) {
    run(bencher, Converter::new().grouping(Grouping::PerCloud));
}

#[divan::bench(sample_count = 10)]
fn las_stable_order(bencher: Bencher) {
    run(bencher, Converter::new().stable_order(true));
}

#[cfg(feature = "laz")]
#[divan::bench(sample_count = 10)]
fn laz_merged(bencher: Bencher) {
    run(bencher, Converter::new().format(Format::Laz));
}

#[cfg(feature = "laz")]
#[divan::bench(sample_count = 10)]
fn laz_per_cloud(bencher: Bencher) {
    run(
        bencher,
        Converter::new()
            .format(Format::Laz)
            .grouping(Grouping::PerCloud),
    );
}

#[divan::bench(sample_count = 10)]
fn scan_metadata(bencher: Bencher) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(TRIMBLE);
    if !path.is_file() {
        return;
    }
    bencher.bench(|| Scan::open(&path).unwrap());
}
