//! gRPC Caching Example with Tonic
//!
//! Demonstrates gRPC-aware response caching by wrapping a tonic gRPC server
//! with the hitbox-tower Cache layer and gRPC-specific predicates/extractors
//! from the `hitbox-grpc` crate.
//!
//! Features shown:
//!   - gRPC service/method request predicates (only cache specific RPCs)
//!   - Cache key extraction from gRPC service name, method name, and metadata headers
//!   - Tonic Server integration via `Server::builder().layer(cache_layer)`
//!
//! Prerequisites:
//!   - `protoc` must be installed (used by tonic-prost-build)
//!
//! Run:
//!   cargo run -p hitbox-examples --example grpc
//!
//! Test with grpcurl:
//!   grpcurl -plaintext \
//!     -import-path examples/proto -proto greeter.proto \
//!     -d '{"name": "World"}' \
//!     localhost:50052 greeter.Greeter/SayHello
//!
//!   # Run again — the timestamp stays frozen (cache HIT):
//!   grpcurl -plaintext \
//!     -import-path examples/proto -proto greeter.proto \
//!     -d '{"name": "World"}' \
//!     localhost:50052 greeter.Greeter/SayHello
//!
//!   # With tenant metadata (different cache key):
//!   grpcurl -plaintext \
//!     -import-path examples/proto -proto greeter.proto \
//!     -H 'x-tenant-id: acme' \
//!     -d '{"name": "World"}' \
//!     localhost:50052 greeter.Greeter/SayHello
//!
//!   # GetTime — also cached:
//!   grpcurl -plaintext \
//!     -import-path examples/proto -proto greeter.proto \
//!     localhost:50052 greeter.Greeter/GetTime

use std::time::Duration;

use tonic::{Request, Response, Status};

use hitbox::Config;
use hitbox::Neutral;
use hitbox::policy::PolicyConfig;
use hitbox_grpc::extractors::metadata::GrpcMetadataExtract;
use hitbox_grpc::extractors::service::GrpcServiceExtractor;
use hitbox_grpc::predicates::service::GrpcService;
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;

// ---------------------------------------------------------------------------
// Generated protobuf types and gRPC server trait
// ---------------------------------------------------------------------------

mod greeter {
    tonic::include_proto!("greeter");
}

use greeter::greeter_server::{Greeter, GreeterServer};
use greeter::{Empty, HelloReply, HelloRequest};

// ---------------------------------------------------------------------------
// Service implementation
// ---------------------------------------------------------------------------

/// gRPC Greeter service implementation.
///
/// Each response includes a timestamp so cache hits are visible — the timestamp
/// freezes when a cached response is served.
struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        // Simulate upstream latency (makes cache hits obvious)
        tokio::time::sleep(Duration::from_millis(200)).await;

        let now = chrono::Utc::now().format("%H:%M:%S%.3f");
        let name = if request.get_ref().name.is_empty() {
            "Anonymous"
        } else {
            &request.get_ref().name
        };

        Ok(Response::new(HelloReply {
            message: format!("Hello, {name}! (served at {now})"),
        }))
    }

    async fn get_time(&self, _request: Request<Empty>) -> Result<Response<HelloReply>, Status> {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let now = chrono::Utc::now().format("%H:%M:%S%.3f");

        Ok(Response::new(HelloReply {
            message: format!("Current time: {now} UTC"),
        }))
    }
}

// ---------------------------------------------------------------------------
// Main — server setup with gRPC-aware cache configuration
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("info,hitbox=debug")
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // In-memory cache backend
    let backend = MokaBackend::builder().max_entries(10_000).build();

    // ----- gRPC-aware cache configuration -----
    //
    // Request predicate:
    //   Only cache calls to greeter.Greeter service, specifically SayHello and GetTime.
    //   Other services or methods (e.g. health checks, mutations) pass through uncached.
    //
    // Response predicate:
    //   Not configured here. Tonic emits grpc-status in HTTP/2 trailers, but the
    //   current GrpcStatus predicate only checks response headers. Since the request
    //   predicate already filters to read-only RPCs, caching all responses is safe.
    //
    // Extractors (cache key components):
    //   grpc_service  = "greeter.Greeter"   (from URI path)
    //   grpc_method   = "SayHello"          (from URI path)
    //   grpc_meta.x-tenant-id = "acme"      (from request metadata header)
    //
    //   This means SayHello for tenant "acme" and tenant "beta" get separate
    //   cache entries, while the same tenant+method combination shares a cache entry.

    let cache_config = Config::builder()
        .request_predicate(
            GrpcService::new("greeter.Greeter").methods(vec!["SayHello".into(), "GetTime".into()]),
        )
        .response_predicate(Neutral::new())
        .extractor(
            GrpcServiceExtractor::new()
                .method()
                .grpc_metadata(vec!["x-tenant-id".into()]),
        )
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(30)).build())
        .build();

    let cache_layer = Cache::builder()
        .backend(backend)
        .config(cache_config)
        .build();

    // Start the tonic gRPC server with the hitbox cache layer
    let addr = "0.0.0.0:50052".parse()?;
    tracing::info!("gRPC server listening on {addr}");
    tracing::info!(
        "Try: grpcurl -plaintext -import-path examples/proto -proto greeter.proto \
         -d '{{\"name\": \"World\"}}' localhost:50052 greeter.Greeter/SayHello"
    );

    tonic::transport::Server::builder()
        .layer(cache_layer)
        .add_service(GreeterServer::new(MyGreeter))
        .serve(addr)
        .await?;

    Ok(())
}
