use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use lsp_server::{Request, RequestId};
use lsp_types::*;
use serde_json::json;
use tokio::runtime::Runtime;

fn create_completion_request(id: i32) -> Request {
    Request {
        id: RequestId::from(id),
        method: "textDocument/completion".to_string(),
        params: json!({
            "textDocument": {
                "uri": "file:///test/sample.txtx"
            },
            "position": {
                "line": 10,
                "character": 6
            }
        }),
    }
}

fn create_hover_request(id: i32) -> Request {
    Request {
        id: RequestId::from(id),
        method: "textDocument/hover".to_string(),
        params: json!({
            "textDocument": {
                "uri": "file:///test/sample.txtx"
            },
            "position": {
                "line": 10,
                "character": 6
            }
        }),
    }
}

fn benchmark_completion(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("lsp_completion_async", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Simulate async completion request
                let req = create_completion_request(1);
                // In a real benchmark, we'd have a proper handler setup
                black_box(req);
            });
        });
    });
}

fn benchmark_hover(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    c.bench_function("lsp_hover_async", |b| {
        b.iter(|| {
            runtime.block_on(async {
                // Simulate async hover request
                let req = create_hover_request(1);
                black_box(req);
            });
        });
    });
}

fn benchmark_concurrent_requests(c: &mut Criterion) {
    let runtime = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_requests");

    for num_requests in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_requests),
            num_requests,
            |b, &num_requests| {
                b.iter(|| {
                    runtime.block_on(async {
                        use futures::future::join_all;

                        let futures = (0..num_requests).map(|i| {
                            async move {
                                let req = if i % 2 == 0 {
                                    create_completion_request(i)
                                } else {
                                    create_hover_request(i)
                                };
                                black_box(req);
                            }
                        });

                        join_all(futures).await;
                    });
                });
            },
        );
    }
    group.finish();
}

fn benchmark_cache_performance(c: &mut Criterion) {
    use std::collections::HashMap;
    use dashmap::DashMap;
    use std::sync::Arc;

    let mut group = c.benchmark_group("cache_performance");

    // Benchmark DashMap (concurrent HashMap)
    group.bench_function("dashmap_insert_get", |b| {
        let map = Arc::new(DashMap::new());
        b.iter(|| {
            for i in 0..100 {
                map.insert(i, format!("value_{}", i));
                black_box(map.get(&i));
            }
        });
    });

    // Benchmark standard HashMap for comparison
    group.bench_function("hashmap_insert_get", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..100 {
                map.insert(i, format!("value_{}", i));
                black_box(map.get(&i));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_completion,
    benchmark_hover,
    benchmark_concurrent_requests,
    benchmark_cache_performance
);
criterion_main!(benches);