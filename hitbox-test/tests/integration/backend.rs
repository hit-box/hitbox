use hitbox_feoxdb::FeOxDbBackend;
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use hitbox_test::backend::run_backend_tests;
use tempfile::TempDir;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::redis::Redis;

#[tokio::test]
async fn test_moka_backend() {
    let backend = MokaBackend::builder(10000).build();
    run_backend_tests(backend).await;
}

#[tokio::test]
async fn test_feoxdb_backend_persistent() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let backend = FeOxDbBackend::open(temp_dir.path()).expect("failed to open FeOxDB");
    run_backend_tests(backend).await;
}

#[tokio::test]
async fn test_feoxdb_backend_in_memory() {
    let backend = FeOxDbBackend::in_memory().expect("failed to create in-memory FeOxDB");
    run_backend_tests(backend).await;
}

#[tokio::test]
async fn test_redis_backend() {
    let container: ContainerAsync<Redis> = Redis::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host = container.get_host().await.expect("failed to get host");
    let host_port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");
    let connection_string = format!("redis://{}:{}", host, host_port);

    let backend = RedisBackend::builder()
        .server(connection_string.clone())
        .build()
        .expect("failed to create Redis backend");

    run_backend_tests(backend).await;
}
