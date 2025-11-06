use axum::{extract::Path, routing::get, Json, Router};
use hitbox_configuration::ConfigEndpoint;
use hitbox_tower::StatusCode;

use hitbox_redis::RedisBackend;
use hitbox_tower::Cache;

async fn handler_result(Path(name): Path<String>) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
    // Err(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handler() -> String {
    //dbg!("axum::handler");
    "root".to_string()
}

#[derive(serde::Serialize)]
struct Greet {
    name: String,
    answer: u32,
}

async fn handler_json() -> Json<Greet> {
    //dbg!("axum::handler");
    Json(Greet {
        name: "root".to_owned(),
        answer: 42,
    })
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let _redis_backend = RedisBackend::new().unwrap();
    // let memory_backend = hitbox_stretto::StrettoBackend::builder(10)
    //     .finalize()
    //     .unwrap();
    let memory_backend = hitbox_moka::MokaBackend::builder(1024 * 1024).build();

    // let json_config = EndpointConfig::builder()
    //     .request(
    //         request::method(Method::GET)
    //             .query("cache", "true")
    //             .query("x-cache", "true")
    //             .path("/{path}*"),
    //     )
    //     .response(response::status_code(StatusCode::OK))
    //     .cache_key(extractor::method().query("cache").path("/{path}*"))
    //     .build();
    let json_cfg = r#"
    request:
    - Method: GET
    - Path: '/{path}*'
    - Query:
        operation: Eq
        cache: 'true'
    extractor:
    - Method
    - Query: cache
    - Path: "/{path}*"
    policy: !Enabled
    "#;
    let json_config = serde_yaml::from_str::<ConfigEndpoint>(json_cfg)
        .unwrap()
        .into_endpoint();

    // let health_config = EndpointConfig::builder()
    //     .request(request::path("/health").method(Method::GET))
    //     .disable()
    //     .build();
    let health_cfg = r#"
    request:
        - Path: "/health"
        - Method: GET
    policy: !Disabled
    "#;
    let health_config = serde_yaml::from_str::<ConfigEndpoint>(health_cfg)
        .unwrap()
        .into_endpoint();

    let json_cache = Cache::builder()
        .backend(memory_backend.clone())
        .config(json_config)
        .build();

    let health_check = Cache::builder()
        .backend(memory_backend) // FIX: it should work without backend
        .config(health_config)
        .build();

    let app = Router::new()
        .route("/greet/{name}", get(handler_result))
        .route("/", get(handler))
        .route("/json/", get(handler_json))
        .route("/health", get(handler).layer(health_check))
        .layer(json_cache);

    // run it with hyper on localhost:3000
    // axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
