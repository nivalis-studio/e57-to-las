use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

// Old implementation for comparison
fn find_smallest_scale_old(x: f64) -> f64 {
    let mut scale = 0.001;
    let min_i32 = f64::from(i32::MIN);
    let max_i32 = f64::from(i32::MAX);

    while (x / scale).round() < min_i32 || (x / scale).round() > max_i32 {
        scale += 0.0001;
    }

    scale
}

// New optimized implementation
fn find_smallest_scale_new(x: f64) -> f64 {
    const MIN_SCALE: f64 = 0.001;
    if x.abs() <= f64::from(i32::MAX) * MIN_SCALE {
        return MIN_SCALE;
    }

    let theoretical_min = x.abs() / f64::from(i32::MAX);
    let scale = (theoretical_min * 10000.0).ceil() / 10000.0;
    scale.max(MIN_SCALE)
}

fn benchmark_scale_finding(c: &mut Criterion) {
    let test_values = [
        ("small", 1000.0),
        ("medium", 2.15e9),
        ("large", 1e10),
        ("very_large", 1e11),
    ];

    let mut group = c.benchmark_group("scale_finding");
    
    for (name, value) in test_values {
        group.bench_with_input(
            BenchmarkId::new("old", name),
            &value,
            |b, &val| b.iter(|| find_smallest_scale_old(black_box(val)))
        );
        
        group.bench_with_input(
            BenchmarkId::new("new", name),
            &value,
            |b, &val| b.iter(|| find_smallest_scale_new(black_box(val)))
        );
    }
    
    group.finish();
}

fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    let values: Vec<f64> = (0..1000)
        .map(|i| 1e6 * (i as f64))
        .collect();
    
    group.bench_function("old_algorithm", |b| {
        b.iter(|| {
            for &val in &values {
                find_smallest_scale_old(black_box(val));
            }
        })
    });
    
    group.bench_function("new_algorithm", |b| {
        b.iter(|| {
            for &val in &values {
                find_smallest_scale_new(black_box(val));
            }
        })
    });
    
    group.finish();
}

criterion_group!(benches, benchmark_scale_finding, benchmark_throughput);
criterion_main!(benches);