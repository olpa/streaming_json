use u8pool::U8Pool;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn bench_sequential_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_add");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("small_elements", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut buffer = vec![0u8; size * 100];
                    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                    for i in 0..size {
                        let data = format!("element_{}", i);
                        black_box(u8pool.add(data.as_bytes()).unwrap());
                    }

                    black_box(u8pool.len())
                });
            },
        );
    }
    group.finish();
}

fn bench_random_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_access");

    for size in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("get_operations", size),
            size,
            |b, &size| {
                let mut buffer = vec![0u8; size * 100];
                let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                // Pre-populate the buffer
                for i in 0..size {
                    let data = format!("element_{}", i);
                    u8pool.add(data.as_bytes()).unwrap();
                }

                b.iter(|| {
                    for i in 0..size {
                        black_box(u8pool.get(i));
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_iterator_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterator");

    for size in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("full_iteration", size),
            size,
            |b, &size| {
                let mut buffer = vec![0u8; size * 100];
                let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                // Pre-populate the buffer
                for i in 0..size {
                    let data = format!("element_{}", i);
                    u8pool.add(data.as_bytes()).unwrap();
                }

                b.iter(|| {
                    for slice in black_box(&u8pool) {
                        black_box(slice);
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_dictionary_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("dictionary");

    for pairs in [50, 500].iter() {
        group.throughput(Throughput::Elements(*pairs as u64));
        group.bench_with_input(
            BenchmarkId::new("pair_iteration", pairs),
            pairs,
            |b, &pairs| {
                let mut buffer = vec![0u8; pairs * 200];
                let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                // Pre-populate with key-value pairs
                for i in 0..pairs {
                    let key = format!("key_{}", i);
                    let value = format!("value_{}", i);
                    u8pool.add(key.as_bytes()).unwrap();
                    u8pool.add(value.as_bytes()).unwrap();
                }

                b.iter(|| {
                    for (key, value) in black_box(u8pool.pairs()) {
                        black_box((key, value));
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_stack_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("stack");

    for size in [100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("push_pop_cycle", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut buffer = vec![0u8; size * 100];
                    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                    // Push elements
                    for i in 0..size {
                        let data = format!("element_{}", i);
                        black_box(u8pool.push(data.as_bytes()).unwrap());
                    }

                    // Pop elements
                    for _ in 0..size {
                        black_box(u8pool.pop());
                    }
                });
            },
        );
    }
    group.finish();
}


fn bench_large_elements(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_elements");

    for element_size in [1024, 4096].iter() {
        group.throughput(Throughput::Bytes(*element_size as u64 * 10));
        group.bench_with_input(
            BenchmarkId::new("add_large", element_size),
            element_size,
            |b, &element_size| {
                b.iter(|| {
                    let mut buffer = vec![0u8; element_size * 20];
                    let mut u8pool = U8Pool::with_default_max_slices(&mut buffer).unwrap();

                    let large_data = vec![b'x'; element_size];

                    for _ in 0..10 {
                        black_box(u8pool.add(&large_data).unwrap());
                    }

                    black_box(u8pool.len())
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_add,
    bench_random_access,
    bench_iterator_performance,
    bench_dictionary_operations,
    bench_stack_operations,
    bench_large_elements
);
criterion_main!(benches);
