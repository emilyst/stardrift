use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

// Import from the crate being benchmarked
extern crate stardrift;
use stardrift::physics::integrators::{Integrator, SymplecticEuler};
use stardrift::physics::math::{Scalar, Vector};

/// Test body data for benchmarking
struct TestBody {
    position: Vector,
    velocity: Vector,
    acceleration: Vector,
}

/// Generate test bodies in a deterministic spiral pattern
fn generate_test_bodies(count: usize) -> Vec<TestBody> {
    let mut bodies = Vec::with_capacity(count);

    for i in 0..count {
        let angle = (i as Scalar) * 2.0 * std::f64::consts::PI / (count as Scalar);
        let radius = 100.0 + (i as Scalar) * 0.1;

        // Position on a spiral
        let position = Vector::new(
            radius * angle.cos(),
            radius * angle.sin(),
            (i as Scalar) * 0.1,
        );

        // Velocity tangent to position (orbital-like)
        let speed = (1.0 / radius.sqrt()) * 10.0;
        let velocity = Vector::new(-speed * angle.sin(), speed * angle.cos(), 0.0);

        // Acceleration towards center (gravity-like)
        let accel_magnitude = 1.0 / (radius * radius) * 100.0;
        let acceleration = -position.normalize() * accel_magnitude;

        bodies.push(TestBody {
            position,
            velocity,
            acceleration,
        });
    }

    bodies
}

fn benchmark_integrators(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrators");

    // Expanded body count range with intermediate values
    for &body_count in &[2, 5, 10, 50, 100, 500, 1000, 2000, 5000, 10000] {
        group.throughput(Throughput::Elements(body_count as u64));

        group.bench_with_input(
            BenchmarkId::new("symplectic_euler", body_count),
            &body_count,
            |b, &count| {
                let integrator = SymplecticEuler;
                let mut bodies = generate_test_bodies(count);
                let dt = 0.01;

                b.iter(|| {
                    for body in bodies.iter_mut() {
                        integrator.integrate_single(
                            &mut body.position,
                            &mut body.velocity,
                            body.acceleration,
                            dt,
                        );
                    }
                    black_box(&bodies);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_integrators);
criterion_main!(benches);
