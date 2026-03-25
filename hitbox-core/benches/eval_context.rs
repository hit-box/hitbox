use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use hitbox_core::EvalContext;
use http::Request;

fn make_request() -> Request<String> {
    Request::builder()
        .method("POST")
        .uri("/api/v1/users/42")
        .header("content-type", "application/json")
        .header("authorization", "Bearer tok_abc123")
        .header("x-request-id", "req-00001")
        .body(r#"{"user_id": 42, "action": "view"}"#.into())
        .unwrap()
}

// === u64 benchmarks ===

fn bench_insert_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();

    c.bench_function("insert/u64/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.insert(42u64).await;
        });
    });
}

fn bench_insert_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());

    c.bench_function("insert/u64/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.insert(42u64).await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_get_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("get/u64/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.get::<u64>().await;
        });
    });
}

fn bench_get_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("get/u64/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.get::<u64>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_get_or_insert_with_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("get_or_insert_with/u64/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.get_or_insert_with(|| async { 42u64 }).await;
        });
    });
}

fn bench_get_or_insert_with_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("get_or_insert_with/u64/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.get_or_insert_with(|| async { 42u64 }).await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_contains_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("contains/u64/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.contains::<u64>().await;
        });
    });
}

fn bench_contains_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(42u64).await });

    c.bench_function("contains/u64/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.contains::<u64>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_remove_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();

    c.bench_function("remove/u64/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.remove::<u64>().await;
        });
    });
}

fn bench_remove_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());

    c.bench_function("remove/u64/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.remove::<u64>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

// === HTTP request benchmarks ===

fn bench_http_insert_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();

    c.bench_function("insert/http_request/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.insert(make_request()).await;
        });
    });
}

fn bench_http_insert_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());

    c.bench_function("insert/http_request/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.insert(make_request()).await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_http_get_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("get/http_request/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.get::<Request<String>>().await;
        });
    });
}

fn bench_http_get_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("get/http_request/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.get::<Request<String>>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_http_get_or_insert_with_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("get_or_insert_with/http_request/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.get_or_insert_with(|| async { make_request() }).await;
        });
    });
}

fn bench_http_get_or_insert_with_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("get_or_insert_with/http_request/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.get_or_insert_with(|| async { make_request() }).await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_http_contains_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("contains/http_request/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.contains::<Request<String>>().await;
        });
    });
}

fn bench_http_contains_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());
    rt.block_on(async { ctx.insert(make_request()).await });

    c.bench_function("contains/http_request/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.contains::<Request<String>>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

fn bench_http_remove_no_contention(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let ctx = EvalContext::new();

    c.bench_function("remove/http_request/no_contention", |b| {
        b.to_async(&rt).iter(|| async {
            ctx.remove::<Request<String>>().await;
        });
    });
}

fn bench_http_remove_100_tasks(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    let ctx = Arc::new(EvalContext::new());

    c.bench_function("remove/http_request/100_tasks_4_threads", |b| {
        b.to_async(&rt).iter(|| {
            let ctx = ctx.clone();
            async move {
                let handles: Vec<_> = (0..100)
                    .map(|_| {
                        let ctx = ctx.clone();
                        tokio::spawn(async move {
                            ctx.remove::<Request<String>>().await;
                        })
                    })
                    .collect();
                for h in handles {
                    h.await.unwrap();
                }
            }
        });
    });
}

criterion_group!(
    benches,
    // u64
    bench_insert_no_contention,
    bench_insert_100_tasks,
    bench_get_no_contention,
    bench_get_100_tasks,
    bench_get_or_insert_with_no_contention,
    bench_get_or_insert_with_100_tasks,
    bench_contains_no_contention,
    bench_contains_100_tasks,
    bench_remove_no_contention,
    bench_remove_100_tasks,
    // HTTP request
    bench_http_insert_no_contention,
    bench_http_insert_100_tasks,
    bench_http_get_no_contention,
    bench_http_get_100_tasks,
    bench_http_get_or_insert_with_no_contention,
    bench_http_get_or_insert_with_100_tasks,
    bench_http_contains_no_contention,
    bench_http_contains_100_tasks,
    bench_http_remove_no_contention,
    bench_http_remove_100_tasks,
);
criterion_main!(benches);
