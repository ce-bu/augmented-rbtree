use augmented_rbtree::{AugmentedRBTree, Unit, augmentations::SubtreeSize};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::{collections::BTreeMap, hint::black_box};

const SIZES: &[i64] = &[100, 1_000, 10_000];

// ============================================================================
// Sequential insert
// ============================================================================

fn bench_insert_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_sequential");
    for &n in SIZES {
        group.bench_with_input(BenchmarkId::new("AugmentedRBTree", n), &n, |b, &n| {
            b.iter(|| {
                let mut tree = AugmentedRBTree::<i64, i64, Unit>::new();
                for i in 0..n {
                    tree.insert(black_box(i), black_box(i));
                }
                black_box(tree)
            });
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, &n| {
            b.iter(|| {
                let mut map = BTreeMap::<i64, i64>::new();
                for i in 0..n {
                    map.insert(black_box(i), black_box(i));
                }
                black_box(map)
            });
        });
    }
    group.finish();
}

// ============================================================================
// Random insert
// ============================================================================

fn bench_insert_random(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_random");
    for &n in SIZES {
        let keys: Vec<i64> = (0..n)
            .map(|i| i.wrapping_mul(6_364_136_223_846_793_005) >> 32)
            .collect();

        group.bench_with_input(BenchmarkId::new("AugmentedRBTree", n), &n, |b, &_n| {
            b.iter(|| {
                let mut tree = AugmentedRBTree::<i64, i64, Unit>::new();
                for &k in &keys {
                    tree.insert(black_box(k), black_box(k));
                }
                black_box(tree)
            });
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, &_n| {
            b.iter(|| {
                let mut map = BTreeMap::<i64, i64>::new();
                for &k in &keys {
                    map.insert(black_box(k), black_box(k));
                }
                black_box(map)
            });
        });
    }
    group.finish();
}

// ============================================================================
// Lookup
// ============================================================================

fn bench_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup");
    for &n in SIZES {
        let keys: Vec<i64> = (0..n).collect();

        let tree: AugmentedRBTree<i64, i64, Unit> = keys.iter().map(|&k| (k, k)).collect();
        let map: BTreeMap<i64, i64> = keys.iter().map(|&k| (k, k)).collect();

        group.bench_with_input(BenchmarkId::new("AugmentedRBTree", n), &n, |b, &_n| {
            b.iter(|| {
                for &k in &keys {
                    black_box(tree.get(black_box(&k)));
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, &_n| {
            b.iter(|| {
                for &k in &keys {
                    black_box(map.get(black_box(&k)));
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// Iteration
// ============================================================================

fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iteration");
    for &n in SIZES {
        let keys: Vec<i64> = (0..n).collect();

        let tree: AugmentedRBTree<i64, i64, Unit> = keys.iter().map(|&k| (k, k)).collect();
        let map: BTreeMap<i64, i64> = keys.iter().map(|&k| (k, k)).collect();

        group.bench_with_input(BenchmarkId::new("AugmentedRBTree", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum: i64 = 0;
                for (_, &v, ()) in &tree {
                    sum = sum.wrapping_add(v);
                }
                black_box(sum)
            });
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum: i64 = 0;
                for &v in map.values() {
                    sum = sum.wrapping_add(v);
                }
                black_box(sum)
            });
        });
    }
    group.finish();
}

// ============================================================================
// Range query
// ============================================================================

fn bench_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_query");
    for &n in SIZES {
        let keys: Vec<i64> = (0..n).collect();
        let tree: AugmentedRBTree<i64, i64, Unit> = keys.iter().map(|&k| (k, k)).collect();
        let map: BTreeMap<i64, i64> = keys.iter().map(|&k| (k, k)).collect();
        let lo = n / 4;
        let hi = n * 3 / 4;

        group.bench_with_input(BenchmarkId::new("AugmentedRBTree", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum: i64 = 0;
                for (_, &v, ()) in tree.range(black_box(lo)..=black_box(hi)) {
                    sum = sum.wrapping_add(v);
                }
                black_box(sum)
            });
        });
        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, &_n| {
            b.iter(|| {
                let mut sum: i64 = 0;
                for &v in map.range(black_box(lo)..=black_box(hi)).map(|(_, v)| v) {
                    sum = sum.wrapping_add(v);
                }
                black_box(sum)
            });
        });
    }
    group.finish();
}

// ============================================================================
// Augmentation overhead (SubtreeSize vs ())
// ============================================================================

fn bench_augmentation_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("augmentation_overhead");
    let n = 1_000i64;
    let keys: Vec<i64> = (0..n)
        .map(|i| i.wrapping_mul(6_364_136_223_846_793_005) >> 32)
        .collect();

    group.bench_function("no_augmentation", |b| {
        b.iter(|| {
            let mut tree = AugmentedRBTree::<i64, i64, Unit>::new();
            for &k in &keys {
                tree.insert(black_box(k), black_box(k));
            }
            black_box(tree)
        });
    });
    group.bench_function("subtree_size", |b| {
        b.iter(|| {
            let mut tree = AugmentedRBTree::<i64, i64, SubtreeSize>::new();
            for &k in &keys {
                tree.insert(black_box(k), black_box(k));
            }
            black_box(tree)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_insert_sequential,
    bench_insert_random,
    bench_lookup,
    bench_iteration,
    bench_range,
    bench_augmentation_overhead,
);
criterion_main!(benches);
