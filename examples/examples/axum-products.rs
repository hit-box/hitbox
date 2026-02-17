//! Product Catalog Caching Example
//!
//! Companion to the blog article "Add Response Caching to Axum in 10 Minutes".
//! Demonstrates all caching patterns covered in the article:
//!
//! - In-memory backend (Moka)
//! - Response predicates (only cache 200 OK)
//! - Cache key extraction from query params and path segments
//! - Auth-aware caching with header hashing
//! - Cache-Control: no-cache bypass
//!
//! Run:
//!   cargo run -p hitbox-examples --example axum-products
//!
//! Endpoints:
//!   GET /products          — Product list (cached, 60s TTL)
//!   GET /products?page=2   — Paginated (separate cache entry)
//!   GET /products/{id}     — Product details (cached, 300s TTL)
//!   GET /health            — Health check (not cached)
//!
//! Try it:
//!   # Basic caching
//!   curl -v http://localhost:3000/products              # MISS
//!   curl -v http://localhost:3000/products              # HIT
//!   curl -v http://localhost:3000/products?page=2       # MISS (different key)
//!
//!   # Per-user caching (auth header hashed in cache key)
//!   curl -v -H 'Authorization: Bearer token-a' http://localhost:3000/products  # MISS
//!   curl -v -H 'Authorization: Bearer token-b' http://localhost:3000/products  # MISS
//!   curl -v -H 'Authorization: Bearer token-a' http://localhost:3000/products  # HIT
//!
//!   # Cache-Control bypass
//!   curl -v -H 'Cache-Control: no-cache' http://localhost:3000/products  # MISS (always)

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox::predicate::PredicateExt;
use hitbox_http::{
    extractors::{
        Method as MethodExtractor,
        header::{Header, NameSelector, Transform, ValueExtractor},
        path::PathExtractor,
        query::QueryExtractor as QueryExtractorTrait,
    },
    predicates::{
        header::{Header as RequestHeader, Operation as HeaderOperation},
        response::StatusCode as ResponseStatusCode,
    },
};
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use serde::{Deserialize, Serialize};

// ── Domain types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct Product {
    id: u32,
    name: String,
    category: String,
    price_cents: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ProductList {
    products: Vec<Product>,
    total: u32,
    page: u32,
}

#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_limit")]
    limit: u32,
    category: Option<String>,
}

fn default_page() -> u32 {
    1
}
fn default_limit() -> u32 {
    20
}

// ── Mock data ───────────────────────────────────────────────────────

fn all_products() -> Vec<Product> {
    vec![
        Product {
            id: 1,
            name: "Mechanical Keyboard".into(),
            category: "peripherals".into(),
            price_cents: 14999,
        },
        Product {
            id: 2,
            name: "USB-C Hub".into(),
            category: "peripherals".into(),
            price_cents: 4999,
        },
        Product {
            id: 3,
            name: "27\" 4K Monitor".into(),
            category: "displays".into(),
            price_cents: 44999,
        },
        Product {
            id: 4,
            name: "Standing Desk".into(),
            category: "furniture".into(),
            price_cents: 59999,
        },
        Product {
            id: 5,
            name: "Ergonomic Mouse".into(),
            category: "peripherals".into(),
            price_cents: 7999,
        },
        Product {
            id: 6,
            name: "Desk Lamp".into(),
            category: "furniture".into(),
            price_cents: 3999,
        },
    ]
}

// ── Handlers ────────────────────────────────────────────────────────

async fn list_products(Query(params): Query<ListParams>) -> Json<ProductList> {
    tracing::info!(
        "DB query: products page={}, category={:?}",
        params.page,
        params.category
    );

    let products = all_products();

    let filtered: Vec<_> = match &params.category {
        Some(cat) => products
            .into_iter()
            .filter(|p| &p.category == cat)
            .collect(),
        None => products,
    };

    let total = filtered.len() as u32;
    let start = ((params.page - 1) * params.limit) as usize;
    let page_items: Vec<_> = filtered
        .into_iter()
        .skip(start)
        .take(params.limit as usize)
        .collect();

    Json(ProductList {
        products: page_items,
        total,
        page: params.page,
    })
}

async fn get_product(Path(id): Path<u32>) -> Result<Json<Product>, http::StatusCode> {
    tracing::info!("DB query: product id={}", id);

    all_products()
        .into_iter()
        .find(|p| p.id == id)
        .map(Json)
        .ok_or(http::StatusCode::NOT_FOUND)
}

async fn health() -> &'static str {
    "OK"
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let backend = MokaBackend::builder().max_entries(10_000).build();

    // Product list: keyed by query params + hashed auth token, 60s TTL
    let list_config = Config::builder()
        .request_predicate(
            // Bypass cache when client sends Cache-Control: no-cache
            RequestHeader::new(HeaderOperation::Contains(
                http::header::CACHE_CONTROL,
                "no-cache".to_string(),
            ))
            .not(),
        )
        .response_predicate(ResponseStatusCode::new(http::StatusCode::OK))
        .extractor(Header::new_with(
            MethodExtractor::new()
                .query("page".to_string())
                .query("limit".to_string())
                .query("category".to_string()),
            NameSelector::Exact("authorization".to_string()),
            ValueExtractor::Full,
            vec![Transform::Hash],
        ))
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    // Product details: keyed by path segment + hashed auth token, 5min TTL
    let details_config = Config::builder()
        .request_predicate(
            RequestHeader::new(HeaderOperation::Contains(
                http::header::CACHE_CONTROL,
                "no-cache".to_string(),
            ))
            .not(),
        )
        .response_predicate(ResponseStatusCode::new(http::StatusCode::OK))
        .extractor(Header::new_with(
            MethodExtractor::new().path("/products/{id}"),
            NameSelector::Exact("authorization".to_string()),
            ValueExtractor::Full,
            vec![Transform::Hash],
        ))
        .policy(
            PolicyConfig::builder()
                .ttl(Duration::from_secs(300))
                .build(),
        )
        .build();

    let list_cache = Cache::builder()
        .backend(backend.clone())
        .config(list_config)
        .build();

    let details_cache = Cache::builder()
        .backend(backend)
        .config(details_config)
        .build();

    let app = Router::new()
        .route("/products", get(list_products).layer(list_cache))
        .route("/products/{id}", get(get_product).layer(details_cache))
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    tracing::info!("Listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("Server error");
}
