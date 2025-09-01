use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

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
        None
    } else if cfg!(target_os = "macos") {
        // On macOS, use ps command
        let output = Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()?;
        
        let rss_str = String::from_utf8_lossy(&output.stdout);
        return rss_str.trim().parse().ok();
    } else if cfg!(target_os = "windows") {
        // Windows - use PowerShell to get memory usage
        let output = Command::new("powershell")
            .args([
                "-Command",
                &format!("(Get-Process -Id {}).WorkingSet64 / 1KB", std::process::id())
            ])
            .output()
            .ok()?;
        
        let mem_str = String::from_utf8_lossy(&output.stdout);
        return mem_str.trim().parse::<f64>().ok().map(|f| f as u64);
    } else {
        None
    }
}

/// Structure to hold benchmark results
#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    duration: Duration,
    memory_before: u64,
    memory_peak: u64,
    memory_after: u64,
}

impl BenchmarkResult {
    fn memory_delta(&self) -> i64 {
        self.memory_peak as i64 - self.memory_before as i64
    }
    
    fn print_summary(&self) {
        println!("\n=== {} Results ===", self.name);
        println!("Duration: {:?}", self.duration);
        println!("Memory before: {} KB", self.memory_before);
        println!("Memory peak: {} KB", self.memory_peak);
        println!("Memory after: {} KB", self.memory_after);
        println!("Memory delta: {} KB", self.memory_delta());
        println!("Memory delta: {:.2} MB", self.memory_delta() as f64 / 1024.0);
    }
}

/// Run a conversion and measure memory and time
fn measure_conversion<F>(name: &str, conversion_fn: F) -> BenchmarkResult
where
    F: FnOnce() -> anyhow::Result<()>,
{
    // Force garbage collection before measurement
    drop(Vec::<u8>::with_capacity(1_000_000));
    std::thread::sleep(Duration::from_millis(100));
    
    let memory_before = get_memory_usage().unwrap_or(0);
    let start = Instant::now();
    
    // Run the conversion
    conversion_fn().expect("Conversion failed");
    
    let duration = start.elapsed();
    let memory_peak = get_memory_usage().unwrap_or(0);
    
    // Clean up and measure final memory
    drop(Vec::<u8>::with_capacity(1));
    std::thread::sleep(Duration::from_millis(100));
    let memory_after = get_memory_usage().unwrap_or(0);
    
    BenchmarkResult {
        name: name.to_string(),
        duration,
        memory_before,
        memory_peak,
        memory_after,
    }
}

/// Compare old vs new implementation
fn comparison_benchmark(c: &mut Criterion) {
    // Only run if we have the test file
    let input_path = "examples/bunnyDouble.e57";
    if !Path::new(input_path).exists() {
        eprintln!("Warning: Test file {} not found. Skipping comparison.", input_path);
        return;
    }

    let mut group = c.benchmark_group("implementation_comparison");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);
    
    // Count points in the file for throughput measurement
    let e57_reader = e57::E57Reader::from_file(input_path).expect("Failed to open e57 file");
    let pointclouds = e57_reader.pointclouds();
    let mut total_points = 0u64;
    
    // This is a rough estimate - actual point count would require reading all clouds
    for _ in pointclouds.iter() {
        total_points += 30000; // Approximate for bunnyDouble.e57
    }
    
    group.throughput(Throughput::Elements(total_points));
    
    // Benchmark streaming (new) implementation
    group.bench_function(BenchmarkId::new("streaming", "implementation"), |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
            let output_path = temp_dir.path().to_str().unwrap().to_string();
            
            let result = measure_conversion("Streaming", || {
                // Use the current streaming implementation
                use e57_to_las::{convert_file, LasVersion};
                convert_file(
                    input_path.to_string(),
                    output_path.clone(),
                    4,
                    true,
                    LasVersion::new(1, 4).unwrap(),
                )
            });
            
            result.print_summary();
            black_box(result);
        });
    });
    
    group.finish();
}

/// Synthetic benchmark to show memory difference more clearly
fn synthetic_memory_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("synthetic_memory");
    
    let sizes = vec![10_000, 100_000, 1_000_000];
    
    for size in sizes {
        // Simulate old implementation (collect all points)
        group.bench_function(BenchmarkId::new("collect_all", size), |b| {
            b.iter(|| {
                let memory_before = get_memory_usage().unwrap_or(0);
                
                // Simulate collecting all points like the old implementation
                let mut points: Vec<(f64, f64, f64, u16, u16, u16)> = Vec::with_capacity(size);
                for i in 0..size {
                    points.push((
                        i as f64,
                        i as f64 * 2.0,
                        i as f64 * 3.0,
                        (i % 65536) as u16,
                        (i % 65536) as u16,
                        (i % 65536) as u16,
                    ));
                }
                
                let memory_after = get_memory_usage().unwrap_or(0);
                let delta = memory_after as i64 - memory_before as i64;
                
                eprintln!("Collect {} points: {} KB ({:.2} MB)", 
                    size, delta, delta as f64 / 1024.0);
                
                black_box(points);
            });
        });
        
        // Simulate new implementation (streaming)
        group.bench_function(BenchmarkId::new("streaming", size), |b| {
            b.iter(|| {
                let memory_before = get_memory_usage().unwrap_or(0);
                
                // Simulate streaming - process one point at a time
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                let mut max_z = f64::NEG_INFINITY;
                
                for i in 0..size {
                    let point = (
                        i as f64,
                        i as f64 * 2.0,
                        i as f64 * 3.0,
                        (i % 65536) as u16,
                        (i % 65536) as u16,
                        (i % 65536) as u16,
                    );
                    
                    max_x = max_x.max(point.0);
                    max_y = max_y.max(point.1);
                    max_z = max_z.max(point.2);
                    
                    // In real implementation, we'd write to file here
                    black_box(point);
                }
                
                let memory_after = get_memory_usage().unwrap_or(0);
                let delta = memory_after as i64 - memory_before as i64;
                
                eprintln!("Stream {} points: {} KB ({:.2} MB)", 
                    size, delta, delta as f64 / 1024.0);
                
                black_box((max_x, max_y, max_z));
            });
        });
    }
    
    group.finish();
}

criterion_group!(benches, comparison_benchmark, synthetic_memory_benchmark);
criterion_main!(benches);