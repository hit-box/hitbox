//! Load test benchmark using oha for throughput and latency measurement.
//!
//! This benchmark tests the full tower integration with hitbox middleware.
//! Configuration is loaded from a YAML file, making it easy to modify
//! predicates/extractors and rerun the test without recompiling.
//!
//! Prerequisites:
//!   cargo install oha
//!
//! Run with:
//!   cargo bench -p hitbox-test --bench load_test -- [OPTIONS]
//!
//! Options:
//!   --config <path>          Config file (default: hitbox-test/benches/fixtures/load_test_config.yaml)
//!   --duration <secs>        Test duration (default: 10)
//!   --connections <n>        Concurrent connections (default: 50)
//!   --requests <n>           Total requests (alternative to duration)
//!   --sleep <ms>             Handler sleep duration in ms (default: 50)
//!   --tui                    Show oha's interactive TUI during test
//!   --http2                  Use HTTP/2 (h2c - cleartext)
//!   --parallel <n>           Parallel requests per connection for HTTP/2 (total workers = connections * parallel)

use std::net::SocketAddr;
use std::process::Command;
use std::time::Duration;

use axum::routing::post;
use axum::{Json, Router};
use hitbox_configuration::{Backend, ConfigEndpoints};
use hitbox_tower::Cache;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

// Reference request body for oha (~2.5KB)
const REQUEST_BODY: &str = include_str!("fixtures/reference_request.json");

// ============================================================================
// CLI Arguments (simple env-based)
// ============================================================================

struct Args {
    config: Option<String>,
    duration: u64,
    connections: u64,
    requests: Option<u64>,
    sleep_ms: u64,
    tui: bool,
    http2: bool,
    parallel: Option<u64>,
}

impl Args {
    fn from_env() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let mut config = None;
        let mut duration = 10u64;
        let mut connections = 50u64;
        let mut requests = None;
        let mut sleep_ms = 50u64;
        let mut tui = false;
        let mut http2 = false;
        let mut parallel = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--config" => {
                    i += 1;
                    if i < args.len() {
                        config = Some(args[i].clone());
                    }
                }
                "--duration" => {
                    i += 1;
                    if i < args.len() {
                        duration = args[i].parse().unwrap_or(10);
                    }
                }
                "--connections" => {
                    i += 1;
                    if i < args.len() {
                        connections = args[i].parse().unwrap_or(50);
                    }
                }
                "--requests" => {
                    i += 1;
                    if i < args.len() {
                        requests = Some(args[i].parse().unwrap_or(1000));
                    }
                }
                "--sleep" => {
                    i += 1;
                    if i < args.len() {
                        sleep_ms = args[i].parse().unwrap_or(50);
                    }
                }
                "--tui" => {
                    tui = true;
                }
                "--http2" => {
                    http2 = true;
                }
                "--parallel" => {
                    i += 1;
                    if i < args.len() {
                        parallel = Some(args[i].parse().unwrap_or(1));
                    }
                }
                _ => {}
            }
            i += 1;
        }

        Args {
            config,
            duration,
            connections,
            requests,
            sleep_ms,
            tui,
            http2,
            parallel,
        }
    }
}

// ============================================================================
// Load Test Configuration
// ============================================================================

#[derive(Debug, Deserialize)]
struct LoadTestConfig {
    backend: Backend,
    endpoints: ConfigEndpoints,
}

// ============================================================================
// Request/Response types (matching reference_request.json structure)
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OrderRequest {
    order: Order,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Order {
    customer_id: String,
    shipping_address: Address,
    billing_address: Address,
    items: Vec<OrderItem>,
    payment: Payment,
    shipping_method: String,
    notes: Option<String>,
    gift_options: Option<GiftOptions>,
    coupon_codes: Vec<String>,
    metadata: Option<OrderMetadata>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Address {
    first_name: String,
    last_name: String,
    company: Option<String>,
    address_line_1: String,
    address_line_2: Option<String>,
    city: String,
    state: String,
    postal_code: String,
    country: String,
    phone: Option<String>,
    email: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OrderItem {
    sku: String,
    product_id: String,
    name: String,
    quantity: u32,
    unit_price: f64,
    discount: f64,
    metadata: Option<ItemMetadata>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ItemMetadata {
    color: Option<String>,
    size: Option<String>,
    material: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Payment {
    method: String,
    card_token: Option<String>,
    billing_descriptor: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct GiftOptions {
    is_gift: bool,
    gift_message: Option<String>,
    gift_wrap: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OrderMetadata {
    source: Option<String>,
    campaign: Option<String>,
    referrer: Option<String>,
    session_id: Option<String>,
}

// Response types (matching reference_response.json structure)
#[derive(Debug, Serialize)]
struct OrderListResponse {
    data: Vec<OrderData>,
    meta: ResponseMeta,
    links: ResponseLinks,
    included: Vec<IncludedCustomer>,
}

#[derive(Debug, Serialize)]
struct OrderData {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    attributes: OrderAttributes,
}

#[derive(Debug, Serialize)]
struct OrderAttributes {
    order_number: String,
    status: String,
    created_at: String,
    updated_at: String,
    total: f64,
    subtotal: f64,
    tax: f64,
    shipping: f64,
    discount: f64,
    currency: String,
    customer: CustomerRef,
    items_count: u32,
    shipping_method: String,
}

#[derive(Debug, Serialize)]
struct CustomerRef {
    id: String,
    email: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct ResponseMeta {
    pagination: Pagination,
    request_id: String,
    processing_time_ms: u32,
    cache_status: String,
    api_version: String,
}

#[derive(Debug, Serialize)]
struct Pagination {
    current_page: u32,
    per_page: u32,
    total_pages: u32,
    total_count: u32,
}

#[derive(Debug, Serialize)]
struct ResponseLinks {
    #[serde(rename = "self")]
    self_: String,
    first: String,
    next: Option<String>,
    last: String,
}

#[derive(Debug, Serialize)]
struct IncludedCustomer {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    attributes: CustomerAttributes,
}

#[derive(Debug, Serialize)]
struct CustomerAttributes {
    email: String,
    name: String,
    created_at: String,
    orders_count: u32,
    total_spent: f64,
}

// ============================================================================
// Test Server
// ============================================================================

#[derive(Clone)]
struct AppState {
    sleep_ms: u64,
}

async fn handle_order(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<OrderRequest>,
) -> Json<OrderListResponse> {
    // Simulate slow backend to demonstrate cache effectiveness
    if state.sleep_ms > 0 {
        tokio::time::sleep(Duration::from_millis(state.sleep_ms)).await;
    }

    // Calculate totals from request
    let subtotal: f64 = req
        .order
        .items
        .iter()
        .map(|i| i.unit_price * i.quantity as f64 - i.discount)
        .sum();
    let tax = subtotal * 0.08;
    let shipping = match req.order.shipping_method.as_str() {
        "express" => 15.0,
        "standard" => 5.0,
        _ => 3.60,
    };
    let discount: f64 = req.order.items.iter().map(|i| i.discount).sum();
    let total = subtotal + tax + shipping;

    // Build response (~5KB)
    let response = OrderListResponse {
        data: vec![
            OrderData {
                id: "order_001".to_string(),
                type_: "order".to_string(),
                attributes: OrderAttributes {
                    order_number: format!("ORD-2024-{}", &req.order.customer_id[5..13]),
                    status: "processing".to_string(),
                    created_at: "2024-01-15T10:30:00Z".to_string(),
                    updated_at: "2024-01-15T14:45:00Z".to_string(),
                    total,
                    subtotal,
                    tax,
                    shipping,
                    discount,
                    currency: "USD".to_string(),
                    customer: CustomerRef {
                        id: req.order.customer_id.clone(),
                        email: req.order.shipping_address.email.clone().unwrap_or_default(),
                        name: format!(
                            "{} {}",
                            req.order.shipping_address.first_name,
                            req.order.shipping_address.last_name
                        ),
                    },
                    items_count: req.order.items.len() as u32,
                    shipping_method: req.order.shipping_method.clone(),
                },
            },
            OrderData {
                id: "order_002".to_string(),
                type_: "order".to_string(),
                attributes: OrderAttributes {
                    order_number: "ORD-2024-001235".to_string(),
                    status: "shipped".to_string(),
                    created_at: "2024-01-14T09:15:00Z".to_string(),
                    updated_at: "2024-01-15T08:30:00Z".to_string(),
                    total: 156.50,
                    subtotal: 139.99,
                    tax: 11.51,
                    shipping: 5.0,
                    discount: 0.0,
                    currency: "USD".to_string(),
                    customer: CustomerRef {
                        id: "cust_660e8401".to_string(),
                        email: "jane.smith@example.com".to_string(),
                        name: "Jane Smith".to_string(),
                    },
                    items_count: 2,
                    shipping_method: "standard".to_string(),
                },
            },
            OrderData {
                id: "order_003".to_string(),
                type_: "order".to_string(),
                attributes: OrderAttributes {
                    order_number: "ORD-2024-001236".to_string(),
                    status: "delivered".to_string(),
                    created_at: "2024-01-10T16:45:00Z".to_string(),
                    updated_at: "2024-01-13T11:20:00Z".to_string(),
                    total: 89.99,
                    subtotal: 79.99,
                    tax: 6.40,
                    shipping: 3.60,
                    discount: 0.0,
                    currency: "USD".to_string(),
                    customer: CustomerRef {
                        id: "cust_770e8402".to_string(),
                        email: "bob.wilson@example.com".to_string(),
                        name: "Bob Wilson".to_string(),
                    },
                    items_count: 1,
                    shipping_method: "economy".to_string(),
                },
            },
        ],
        meta: ResponseMeta {
            pagination: Pagination {
                current_page: 1,
                per_page: 20,
                total_pages: 15,
                total_count: 294,
            },
            request_id: "req-550e8400-e29b-41d4-a716-446655440000".to_string(),
            processing_time_ms: 42,
            cache_status: "miss".to_string(),
            api_version: "2024-01-01".to_string(),
        },
        links: ResponseLinks {
            self_: "/v1/orders?page=1&per_page=20".to_string(),
            first: "/v1/orders?page=1&per_page=20".to_string(),
            next: Some("/v1/orders?page=2&per_page=20".to_string()),
            last: "/v1/orders?page=15&per_page=20".to_string(),
        },
        included: vec![
            IncludedCustomer {
                id: req.order.customer_id.clone(),
                type_: "customer".to_string(),
                attributes: CustomerAttributes {
                    email: req.order.shipping_address.email.clone().unwrap_or_default(),
                    name: format!(
                        "{} {}",
                        req.order.shipping_address.first_name, req.order.shipping_address.last_name
                    ),
                    created_at: "2023-06-15T00:00:00Z".to_string(),
                    orders_count: 12,
                    total_spent: 1542.50,
                },
            },
            IncludedCustomer {
                id: "cust_660e8401".to_string(),
                type_: "customer".to_string(),
                attributes: CustomerAttributes {
                    email: "jane.smith@example.com".to_string(),
                    name: "Jane Smith".to_string(),
                    created_at: "2023-08-20T00:00:00Z".to_string(),
                    orders_count: 5,
                    total_spent: 678.25,
                },
            },
        ],
    };

    Json(response)
}

async fn start_server_with_cache(
    config_path: &str,
    sleep_ms: u64,
    http2: bool,
) -> anyhow::Result<SocketAddr> {
    // Read config from file (handle both relative and absolute paths)
    let config_yaml = std::fs::read_to_string(config_path)
        .or_else(|_| {
            // Try from workspace root
            let workspace_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join(config_path);
            std::fs::read_to_string(&workspace_path)
        })
        .or_else(|_| {
            // Try from hitbox-test directory
            let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                config_path
                    .strip_prefix("hitbox-test/")
                    .unwrap_or(config_path),
            );
            std::fs::read_to_string(&manifest_path)
        })?;

    // Parse combined config
    let config: LoadTestConfig = serde_saphyr::from_str(&config_yaml)?;

    // Create backend from config
    let backend = config.backend.into_backend()?;

    // Create endpoint config (use first endpoint from the list)
    let endpoint_config = config
        .endpoints
        .into_iter()
        .next()
        .expect("endpoints list must not be empty")
        .into_endpoint()?;

    // Create cache layer
    let cache_layer = Cache::builder()
        .backend(backend)
        .config(endpoint_config)
        .build();

    let state = AppState { sleep_ms };
    let app = Router::new()
        .route("/api/orders", post(handle_order))
        .layer(cache_layer)
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    if http2 {
        serve_http2(listener, app.into_make_service()).await;
    } else {
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok(addr)
}

async fn start_server_without_cache(sleep_ms: u64, http2: bool) -> SocketAddr {
    let state = AppState { sleep_ms };
    let app = Router::new()
        .route("/api/orders", post(handle_order))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    if http2 {
        serve_http2(listener, app.into_make_service()).await;
    } else {
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    addr
}

async fn serve_http2<S>(listener: TcpListener, make_service: S)
where
    S: tower::MakeService<
            SocketAddr,
            http::Request<hyper::body::Incoming>,
            Response = axum::response::Response,
            Error = std::convert::Infallible,
        > + Send
        + 'static,
    S::Service: Send + Clone,
    S::Future: Send,
    <S::Service as tower::Service<http::Request<hyper::body::Incoming>>>::Future: Send,
{
    use hyper_util::rt::{TokioExecutor, TokioIo};
    use hyper_util::server::conn::auto::Builder;
    use hyper_util::service::TowerToHyperService;

    let mut make_service = make_service;

    tokio::spawn(async move {
        loop {
            let (socket, remote_addr) = listener.accept().await.unwrap();
            let io = TokioIo::new(socket);

            let tower_service = match make_service.make_service(remote_addr).await {
                Ok(s) => s,
                Err(_) => continue,
            };
            let service = TowerToHyperService::new(tower_service);

            tokio::spawn(async move {
                let builder = Builder::new(TokioExecutor::new());
                if let Err(err) = builder.serve_connection_with_upgrades(io, service).await {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    });
}

// ============================================================================
// Load Test Runner (using oha subprocess)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OhaResult {
    summary: OhaSummary,
    latency_percentiles: OhaLatencyPercentiles,
    status_code_distribution: std::collections::HashMap<String, u64>,
    error_distribution: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OhaSummary {
    success_rate: f64,
    total: f64,
    slowest: f64,
    fastest: f64,
    average: f64,
    requests_per_sec: f64,
    total_data: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OhaLatencyPercentiles {
    p10: f64,
    p25: f64,
    p50: f64,
    p75: f64,
    p90: f64,
    p95: f64,
    p99: f64,
    #[serde(rename = "p99.9")]
    p99_9: f64,
    #[serde(rename = "p99.99")]
    p99_99: f64,
}

fn run_oha(addr: SocketAddr, args: &Args) -> anyhow::Result<OhaResult> {
    let url = format!("http://{}/api/orders?include=items,customer", addr);

    // Build oha command with reference request body (~2.5KB)
    let mut cmd = Command::new("oha");
    cmd.arg(&url)
        .arg("-m")
        .arg("POST")
        .arg("-T")
        .arg("application/json")
        .arg("-d")
        .arg(REQUEST_BODY)
        .arg("-H")
        .arg("x-tenant-id: tenant-abc")
        .arg("-H")
        .arg("content-type: application/json")
        .arg("-c")
        .arg(args.connections.to_string())
        .arg("--output-format")
        .arg("json"); // JSON output to stdout

    // Only disable TUI if not requested
    if !args.tui {
        cmd.arg("--no-tui");
    }

    // Enable HTTP/2 prior knowledge (h2c) if requested or if parallel is set
    if args.http2 || args.parallel.is_some() {
        cmd.arg("--http2");
    }

    // Set parallel requests per connection (HTTP/2)
    if let Some(p) = args.parallel {
        cmd.arg("-p").arg(p.to_string());
    }

    if let Some(n) = args.requests {
        cmd.arg("-n").arg(n.to_string());
    } else {
        cmd.arg("-z").arg(format!("{}s", args.duration));
    }

    // Run oha - with TUI we use -o flag to write JSON to file while TUI renders normally
    let output = if args.tui {
        // Create temp file path for JSON output
        let temp_path =
            std::env::temp_dir().join(format!("oha_result_{}.json", std::process::id()));

        // Use oha's -o flag to write JSON to file, TUI renders to terminal
        cmd.arg("-o").arg(&temp_path);

        let status = cmd.status()?;

        if !status.success() {
            let _ = std::fs::remove_file(&temp_path);
            anyhow::bail!("oha failed with status: {}", status);
        }

        // Read JSON from temp file
        let json = std::fs::read_to_string(&temp_path)?;
        let _ = std::fs::remove_file(&temp_path);
        json
    } else {
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("oha failed: {}", stderr);
        }
        String::from_utf8(output.stdout)?
    };

    // Parse JSON output
    let result: OhaResult = serde_json::from_str(&output)?;

    Ok(result)
}

fn format_rps(rps: f64) -> String {
    if rps >= 1_000_000.0 {
        format!("{:.2}M", rps / 1_000_000.0)
    } else if rps >= 1_000.0 {
        format!("{:.2}K", rps / 1_000.0)
    } else {
        format!("{:.2}", rps)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn print_results(name: &str, result: &OhaResult) {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  {}", name);
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("  Summary:");
    println!(
        "    Success rate:    {:.2}%",
        result.summary.success_rate * 100.0
    );
    println!("    Total duration:  {:.2}s", result.summary.total);
    println!(
        "    Requests/sec:    {} req/s",
        format_rps(result.summary.requests_per_sec)
    );
    println!(
        "    Data transferred: {}",
        format_bytes(result.summary.total_data)
    );
    println!();
    println!("  Latency:");
    println!("    Fastest:  {:.3}ms", result.summary.fastest * 1000.0);
    println!("    Average:  {:.3}ms", result.summary.average * 1000.0);
    println!("    Slowest:  {:.3}ms", result.summary.slowest * 1000.0);
    println!();
    println!("  Percentiles:");
    println!(
        "    p50:      {:.3}ms",
        result.latency_percentiles.p50 * 1000.0
    );
    println!(
        "    p90:      {:.3}ms",
        result.latency_percentiles.p90 * 1000.0
    );
    println!(
        "    p95:      {:.3}ms",
        result.latency_percentiles.p95 * 1000.0
    );
    println!(
        "    p99:      {:.3}ms",
        result.latency_percentiles.p99 * 1000.0
    );
    println!(
        "    p99.9:    {:.3}ms",
        result.latency_percentiles.p99_9 * 1000.0
    );
    println!();
    println!("  Status codes:");
    for (code, count) in &result.status_code_distribution {
        println!("    {}: {}", code, count);
    }
    if !result.error_distribution.is_empty() {
        println!();
        println!("  Errors:");
        for (err, count) in &result.error_distribution {
            println!("    {}: {}", err, count);
        }
    }
    println!();
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::from_env();

    // Check if oha is installed
    if Command::new("oha").arg("--version").output().is_err() {
        eprintln!("Error: oha is not installed. Install with: cargo install oha");
        std::process::exit(1);
    }

    let config_path = args
        .config
        .as_deref()
        .unwrap_or("hitbox-test/benches/fixtures/load_test_config.yaml");

    println!();
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║              Hitbox Load Test Benchmark                       ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Configuration:");
    println!("  Config:      {}", config_path);
    println!("  Duration:    {}s", args.duration);
    println!("  Connections: {}", args.connections);
    if let Some(n) = args.requests {
        println!("  Requests:    {}", n);
    }
    println!("  Sleep:       {}ms", args.sleep_ms);
    // HTTP/2 is enabled if explicitly requested or if parallel is set
    let use_http2 = args.http2 || args.parallel.is_some();
    println!(
        "  Protocol:    {}",
        if use_http2 { "HTTP/2" } else { "HTTP/1.1" }
    );
    if let Some(p) = args.parallel {
        println!(
            "  Parallel:    {} (total workers: {})",
            p,
            args.connections * p
        );
    }
    println!();

    // Run baseline (no cache)
    println!("Starting baseline server (no cache)...");
    let addr_baseline = start_server_without_cache(args.sleep_ms, use_http2).await;
    println!("  Server running at {}", addr_baseline);

    println!("Running baseline load test...");
    let baseline_result = run_oha(addr_baseline, &args)?;
    print_results("Baseline (No Cache)", &baseline_result);

    // Run with cache
    println!("Starting cached server...");
    let addr_cached = start_server_with_cache(config_path, args.sleep_ms, use_http2).await?;
    println!("  Server running at {}", addr_cached);

    println!("Running cached load test...");
    let cached_result = run_oha(addr_cached, &args)?;
    print_results("With Hitbox Cache", &cached_result);

    // Comparison
    println!("═══════════════════════════════════════════════════════════════");
    println!("  Comparison");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    let speedup = cached_result.summary.requests_per_sec / baseline_result.summary.requests_per_sec;
    let latency_improvement = baseline_result.summary.average / cached_result.summary.average;
    println!("  Throughput improvement: {:.2}x", speedup);
    println!("  Latency improvement:    {:.2}x", latency_improvement);
    println!();

    Ok(())
}
