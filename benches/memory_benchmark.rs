use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use e57_to_las::{convert_pointcloud, LasVersion};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Get current memory usage in KB for the current process
fn get_memory_usage() -> Option<u64> {
    if cfg!(target_os = "linux") {
        // On Linux, read from /proc/self/status
        let status = fs::read_to_string("/proc/self/status").ok()?;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }
    } else if cfg!(target_os = "macos") {
        // On macOS, use ps command
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        
        let rss_str = String::from_utf8_lossy(&output.stdout);
        return rss_str.trim().parse().ok();
    } else {
        // Windows or other platforms - would need different implementation
        return None;
    }
    #[allow(unreachable_code)]
    None
}

/// Benchmark function that also tracks memory usage
fn benchmark_with_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("e57_conversion");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    // Use the example bunnyDouble.e57 file for benchmarking
    let input_path = "examples/bunnyDouble.e57";
    
    // Check if the test file exists
    if !Path::new(input_path).exists() {
        eprintln!("Warning: Test file {} not found. Skipping benchmarks.", input_path);
        eprintln!("To run benchmarks, ensure the example E57 file is available.");
        return;
    }

    // Create a temporary directory for output
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let output_path = temp_dir.path().to_str().unwrap().to_string();

    // Benchmark the streaming version (current implementation)
    group.bench_function(BenchmarkId::new("streaming", "bunnyDouble"), |b| {
        b.iter(|| {
            let memory_before = get_memory_usage();
            
            // Initialize thread pool for each iteration to ensure consistent state
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build_global()
                .ok();

            let e57_reader = e57::E57Reader::from_file(input_path)
                .expect("Failed to open e57 file");
            
            let pointclouds = e57_reader.pointclouds();
            let las_version = LasVersion::new(1, 4).unwrap();
            
            for (index, pointcloud) in pointclouds.iter().enumerate() {
                convert_pointcloud(
                    black_box(index),
                    black_box(pointcloud),
                    black_box(&input_path.to_string()),
                    black_box(&output_path),
                    black_box(&las_version),
                )
                .expect("Conversion failed");
            }

            let memory_after = get_memory_usage();
            
            // Print memory usage for manual inspection
            if let (Some(before), Some(after)) = (memory_before, memory_after) {
                let diff = after as i64 - before as i64;
                eprintln!("Memory delta: {} KB", diff);
            }

            // Clean up output files
            let _ = fs::remove_dir_all(temp_dir.path().join("las"));
        });
    });

    group.finish();
}

/// Memory usage tracking benchmark
fn memory_usage_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    // This benchmark specifically tracks peak memory usage
    group.bench_function("peak_memory", |b| {
        b.iter(|| {
            let start_memory = get_memory_usage().unwrap_or(0);
            
            // Allocate some memory to simulate point cloud processing
            let size = 1_000_000; // 1 million points
            let mut points: Vec<(f64, f64, f64)> = Vec::with_capacity(size);
            
            for i in 0..size {
                points.push((i as f64, i as f64 * 2.0, i as f64 * 3.0));
            }
            
            let peak_memory = get_memory_usage().unwrap_or(0);
            let memory_used = peak_memory - start_memory;
            
            eprintln!("Peak memory for {} points: {} KB", size, memory_used);
            
            black_box(points);
        });
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_with_memory, memory_usage_benchmark);
criterion_main!(benches);