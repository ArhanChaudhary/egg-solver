//! Micro-benchmarks of the search's kernels (criterion; not in the default
//! build): `cargo bench --features bench`. Each runs over a ring of 1024
//! random cubes, so that its inputs are not constants. These time a kernel
//! alone: the solver's speed comes from interleaving them across lanes
//! (`bench` measures that).

use criterion::{criterion_group, criterion_main, Criterion};
use cubeopt::cubie::{CubieCube, Rng};
use cubeopt::simd::Cube;
use cubeopt::solver::Solver;
use cubeopt::tables::Cocsep;
use std::hint::black_box;

/// 1024 random cubes.
fn cubes() -> Vec<Cube> {
    let mut rng = Rng(2026);
    (0..1024)
        .map(|_| Cube::from_cubie(&CubieCube::random(&mut rng)))
        .collect()
}

fn kernels(c: &mut Criterion) {
    let cubes = cubes();
    let corners = Cocsep::generate();
    let mut i = 0usize;
    let mut next = move || {
        i = (i + 1) % 1024;
        i
    };
    c.bench_function("apply_move", |b| {
        b.iter(|| {
            let i = next();
            black_box(cubes[i]).apply_move(black_box(i % 18))
        });
    });
    c.bench_function("child_lookup", |b| {
        b.iter(|| {
            let i = next();
            let mask = if i & 1 == 0 { 0 } else { u32::MAX };
            // SAFETY: the move is below 18.
            unsafe {
                Cube::child_lookup(
                    black_box(cubes[i]),
                    black_box(cubes[1023 - i]),
                    black_box(i % 18),
                    black_box(mask),
                )
            }
        });
    });
    c.bench_function("index (h11 key, reference path)", |b| {
        b.iter(|| corners.index(black_box(cubes[next()]), 11));
    });
    // (h0, made in memory: a few seconds.)
    let table = cubeopt::generate_table(&corners, 0, 8, cubeopt::thread_count(), false);
    let solver = Solver::new(&corners, &table);
    c.bench_function("lower_bound (index and h0 lookup)", |b| {
        b.iter(|| solver.lower_bound(black_box(cubes[next()])));
    });
}

criterion_group!(benches, kernels);
criterion_main!(benches);
