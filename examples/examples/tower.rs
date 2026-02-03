//! Tower Service Example
//!
//! Demonstrates hitbox-tower Cache layer with a generic Tower service and Hyper server.
//!
//! Features shown:
//!   - Direct tower::Service trait implementation
//!   - Hyper HTTP server integration (without Axum)
//!   - TowerToHyperService adapter pattern
//!   - Path-based request routing
//!
//! Run:
//!   cargo run -p hitbox-examples --example tower
//!
//! Endpoints:
//!   - http://localhost:3001/         - Hello World (cached, TTL: 30s)
//!   - http://localhost:3001/time     - Current timestamp (cached, TTL: 5s)
//!   - http://localhost:3001/health   - Health check (not cached)
//!
//! Try it:
//!   curl -v http://localhost:3001/           # Cache miss, then hit
//!   curl -v http://localhost:3001/time       # Shows cached time
//!   curl -v http://localhost:3001/health     # Always fresh

use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::{self, MethodConfig, MethodExtractor, PathExtractor},
    predicates::{request, request::MethodPredicate, response},
};
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tower::{Service, ServiceBuilder};

/// Simple handler service that routes requests based on path
#[derive(Clone)]
struct HelloService;

impl<B> Service<Request<B>> for HelloService
where
    B: Send + 'static,
{
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let path = req.uri().path().to_string();

        Box::pin(async move {
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(50)).await;

            let response = match path.as_str() {
                "/" => Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/plain")
                    .body(Full::new(Bytes::from("Hello from Tower + Hitbox!")))
                    .unwrap(),

                "/time" => {
                    let now = chrono::Utc::now().to_rfc3339();
                    Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "text/plain")
                        .body(Full::new(Bytes::from(format!("Current time: {}", now))))
                        .unwrap()
                }

                "/health" => Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/plain")
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap(),

                _ => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "text/plain")
                    .body(Full::new(Bytes::from("Not Found")))
                    .unwrap(),
            };

            Ok(response)
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize tracing
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("info,hitbox=debug")
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Create Moka in-memory cache backend
    let backend = MokaBackend::builder().max_entries(10_000).build();

    // Cache configuration for all cacheable endpoints
    // - Only cache GET requests
    // - Cache key includes: method + path
    // - TTL: 30 seconds
    let cache_config = Config::builder()
        .request_predicate(request::predicate().method(http::Method::GET))
        .response_predicate(response::predicate())
        .extractor(
            extractors::extractor()
                .method(MethodConfig::new())
                .path("/{path}*"),
        )
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(30)).build())
        .build();

    // Build the cache layer
    let cache_layer = Cache::builder()
        .backend(backend)
        .config(cache_config)
        .build();

    // Create the service stack with caching
    let service = ServiceBuilder::new()
        .layer(cache_layer)
        .service(HelloService);

    // Bind to address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on http://{}", addr);

    // Accept connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let svc = service.clone();

        tokio::task::spawn(async move {
            // Convert tower service to hyper service
            let hyper_service = TowerToHyperService::new(svc);

            if let Err(err) = http1::Builder::new()
                .serve_connection(io, hyper_service)
                .await
            {
                tracing::error!(?err, "Error serving connection");
            }
        });
    }
}
