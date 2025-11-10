#[cfg(feature = "gzip")]
use hitbox_backend::GzipCompressor;
#[cfg(feature = "zstd")]
use hitbox_backend::ZstdCompressor;
use hitbox_backend::serializer::{BincodeFormat, Format, JsonFormat};
use hitbox_backend::{Backend, CacheBackend, CacheKeyFormat, Compressor, PassthroughCompressor};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use hitbox_test::backend::{
    test_bitcode_key_bincode_value, test_bitcode_key_json_value,
    test_url_encoded_key_bincode_value, test_url_encoded_key_json_value,
};
use std::sync::{Arc, LazyLock};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::redis::Redis;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValueFormat {
    Json,
    Bincode,
}

type FormatCombination = (CacheKeyFormat, ValueFormat, Arc<dyn Compressor>);

/// All supported format combinations (key format, value format, compressor)
static FORMAT_COMBINATIONS: LazyLock<Vec<FormatCombination>> = LazyLock::new(|| {
    let mut combinations = Vec::new();

    let key_formats = [CacheKeyFormat::UrlEncoded, CacheKeyFormat::Bitcode];
    let value_formats = [ValueFormat::Json, ValueFormat::Bincode];

    for &key_format in &key_formats {
        for &value_format in &value_formats {
            // No compression
            combinations.push((
                key_format,
                value_format,
                Arc::new(PassthroughCompressor) as Arc<dyn Compressor>,
            ));

            // Gzip compression
            #[cfg(feature = "gzip")]
            combinations.push((
                key_format,
                value_format,
                Arc::new(GzipCompressor::with_level(6)) as Arc<dyn Compressor>,
            ));

            // Zstd compression
            #[cfg(feature = "zstd")]
            combinations.push((
                key_format,
                value_format,
                Arc::new(ZstdCompressor::with_level(3)) as Arc<dyn Compressor>,
            ));
        }
    }

    combinations
});

fn value_format_to_serializer(format: ValueFormat) -> Arc<dyn Format> {
    match format {
        ValueFormat::Json => Arc::new(JsonFormat),
        ValueFormat::Bincode => Arc::new(BincodeFormat),
    }
}

/// Test all format combinations for a given backend
async fn test_all_formats<B>(backend: &B, key_format: CacheKeyFormat, value_format: ValueFormat)
where
    B: Backend + CacheBackend,
{
    assert_eq!(backend.key_format(), &key_format);

    match (key_format, value_format) {
        (CacheKeyFormat::UrlEncoded, ValueFormat::Json) => {
            test_url_encoded_key_json_value(backend).await
        }
        (CacheKeyFormat::UrlEncoded, ValueFormat::Bincode) => {
            test_url_encoded_key_bincode_value(backend).await
        }
        (CacheKeyFormat::Bitcode, ValueFormat::Json) => test_bitcode_key_json_value(backend).await,
        (CacheKeyFormat::Bitcode, ValueFormat::Bincode) => {
            test_bitcode_key_bincode_value(backend).await
        }
    }
}

#[tokio::test]
async fn test_moka_all_format_combinations() {
    for (key_format, value_format, compressor) in FORMAT_COMBINATIONS.iter() {
        let backend = MokaBackend::builder(1000)
            .key_format(*key_format)
            .value_format(value_format_to_serializer(*value_format))
            .compressor(compressor.clone())
            .build();

        test_all_formats(&backend, *key_format, *value_format).await;
    }
}

#[tokio::test]
async fn test_feoxdb_all_format_combinations() {
    for (key_format, value_format, compressor) in FORMAT_COMBINATIONS.iter() {
        let backend = FeOxDbBackend::builder()
            .key_format(*key_format)
            .value_format(value_format_to_serializer(*value_format))
            .compressor(compressor.clone())
            .build()
            .expect("failed to create backend");

        test_all_formats(&backend, *key_format, *value_format).await;
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

    for (key_format, value_format, compressor) in FORMAT_COMBINATIONS.iter() {
        let backend = RedisBackend::builder()
            .server(format!("redis://127.0.0.1:{}", host_port))
            .key_format(*key_format)
            .value_format(value_format_to_serializer(*value_format))
            .compressor(compressor.clone())
            .build()
            .expect("failed to create backend");

        test_all_formats(&backend, *key_format, *value_format).await;
    }
}
