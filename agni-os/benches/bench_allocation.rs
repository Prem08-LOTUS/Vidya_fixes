use criterion::{black_box, criterion_group, criterion_main, Criterion};
use agnix::persistence::MotionIntent;
use agnix::types::ControlCommand;
use agnix::physics::types::Nanometers;
use std::sync::Arc;

fn benchmark_allocation(c: &mut Criterion) {
    let poly_size = 1000;
    // Pre-allocate vector to clone from
    let poly: Vec<(f64, f64)> = (0..poly_size).map(|i| (i as f64, i as f64)).collect();

    c.bench_function("optimized_allocation", |b| {
        b.iter(|| {
            // Simulate getting a polygon from parser
            let p = poly.clone();
            let p_arc = Arc::new(p); // Wrap in Arc

            // Optimized:
            // 1. Cheap clone for Intent
            let intent = MotionIntent::ScanSegment {
                vertices: p_arc.clone(),
                velocity_um_s: 50.0,
            };

            // 2. Cheap move/clone for Command
            let cmd = ControlCommand::ScanPath {
                points: p_arc,
                velocity_um_s: 50.0,
                intent_id: "test".to_string(),
            };

            black_box((intent, cmd));
        })
    });
}

criterion_group!(benches, benchmark_allocation);
criterion_main!(benches);
