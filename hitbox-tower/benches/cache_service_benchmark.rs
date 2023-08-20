use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};

// This is a struct that tells Criterion.rs to use the "futures" crate's current-thread executor
use tokio::runtime::Runtime;

// Here we have an async function to benchmark
async fn do_something(size: usize) {
    // Do something async with the size
}

fn from_elem(c: &mut Criterion) {
    let size: usize = 1024;

    c.bench_with_input(BenchmarkId::new("input_example", size), &size, |b, &s| {
        // Insert a call to `to_async` to convert the bencher to async mode.
        // The timing loops are the same as with the normal bencher.
        b.to_async(Runtime::new().unwrap()).iter(|| do_something(s));
    });
}

criterion_group!(benches, from_elem);
criterion_main!(benches);
