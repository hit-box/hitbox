use hitbox_backend::serializer::Format;
use hitbox_backend::{Backend, CacheBackend, CacheKeyFormat};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use hitbox_test::backend::{
    test_bitcode_key_bincode_value, test_bitcode_key_json_value,
    test_url_encoded_key_bincode_value, test_url_encoded_key_json_value,
};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::Redis;

/// All supported format combinations
const FORMAT_COMBINATIONS: [(CacheKeyFormat, Format); 4] = [
    (CacheKeyFormat::UrlEncoded, Format::Json),
    (CacheKeyFormat::UrlEncoded, Format::Bincode),
    (CacheKeyFormat::Bitcode, Format::Json),
    (CacheKeyFormat::Bitcode, Format::Bincode),
];

/// Test all format combinations for a given backend
async fn test_all_formats<B>(backend: &B)
where
    B: Backend + CacheBackend,
{
    match (backend.key_format(), backend.value_format()) {
        (&CacheKeyFormat::UrlEncoded, &Format::Json) => {
            test_url_encoded_key_json_value(backend).await
        }
        (&CacheKeyFormat::UrlEncoded, &Format::Bincode) => {
            test_url_encoded_key_bincode_value(backend).await
        }
        (&CacheKeyFormat::Bitcode, &Format::Json) => test_bitcode_key_json_value(backend).await,
        (&CacheKeyFormat::Bitcode, &Format::Bincode) => {
            test_bitcode_key_bincode_value(backend).await
        }
    }
}

#[tokio::test]
async fn test_moka_all_format_combinations() {
    for (key_format, value_format) in FORMAT_COMBINATIONS {
        let backend = MokaBackend::builder(1000)
            .key_format(key_format)
            .value_format(value_format)
            .build();

        test_all_formats(&backend).await;
    }
}

#[tokio::test]
async fn test_feoxdb_all_format_combinations() {
    for (key_format, value_format) in FORMAT_COMBINATIONS {
        let backend = FeOxDbBackend::builder()
            .key_format(key_format)
            .value_format(value_format)
            .build()
            .expect("failed to create backend");

        test_all_formats(&backend).await;
    }
}

#[tokio::test]
async fn test_redis_all_format_combinations() {
    let container = Redis::default()
        .start()
        .await
        .expect("failed to start redis");
    let host_port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");

    for (key_format, value_format) in FORMAT_COMBINATIONS {
        let backend = RedisBackend::builder()
            .server(format!("redis://127.0.0.1:{}", host_port))
            .key_format(key_format)
            .value_format(value_format)
            .build()
            .expect("failed to create backend");

        test_all_formats(&backend).await;
    }
}
