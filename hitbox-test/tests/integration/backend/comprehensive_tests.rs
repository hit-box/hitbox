use hitbox_backend::{Backend, CacheBackend};
use hitbox_configuration::backend::{
    BackendConfig, Compression, FeOxDb, KeyFormat, KeySerialization, Moka, Redis, ValueFormat,
    ValueSerialization,
};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_moka::MokaBackend;
use hitbox_redis::RedisBackend;
use hitbox_test::backend::{
    run_backend_tests, test_bitcode_key_bincode_value, test_bitcode_key_json_value,
    test_compression_is_used, test_url_encoded_key_bincode_value, test_url_encoded_key_json_value,
};
use std::sync::LazyLock;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::redis::Redis as RedisContainer;

/// All test configurations (key format, value format, compression)
static TEST_CONFIGS: LazyLock<Vec<(KeySerialization, ValueSerialization, Compression)>> =
    LazyLock::new(|| {
        let mut configs = Vec::new();

        let key_formats = [KeySerialization::UrlEncoded, KeySerialization::Bitcode];
        let value_formats = [ValueSerialization::Json, ValueSerialization::Bincode];

        for key_format in &key_formats {
            for value_format in &value_formats {
                // No compression
                configs.push((
                    key_format.clone(),
                    value_format.clone(),
                    Compression::Disabled,
                ));

                // Gzip compression
                #[cfg(feature = "gzip")]
                configs.push((
                    key_format.clone(),
                    value_format.clone(),
                    Compression::Gzip { level: 6 },
                ));

                // Zstd compression
                #[cfg(feature = "zstd")]
                configs.push((
                    key_format.clone(),
                    value_format.clone(),
                    Compression::Zstd { level: 3 },
                ));
            }
        }

        configs
    });

/// Run comprehensive backend tests for a given configuration
async fn run_comprehensive_backend_tests<B>(
    backend: &B,
    key_format: &KeySerialization,
    value_format: &ValueSerialization,
    compression: &Compression,
) where
    B: Backend + CacheBackend,
{
    // Run basic CRUD tests
    run_backend_tests(backend).await;

    // Run format-specific validation tests
    match (key_format, value_format) {
        (KeySerialization::UrlEncoded, ValueSerialization::Json) => {
            test_url_encoded_key_json_value(backend).await
        }
        (KeySerialization::UrlEncoded, ValueSerialization::Bincode) => {
            test_url_encoded_key_bincode_value(backend).await
        }
        (KeySerialization::Bitcode, ValueSerialization::Json) => {
            test_bitcode_key_json_value(backend).await
        }
        (KeySerialization::Bitcode, ValueSerialization::Bincode) => {
            test_bitcode_key_bincode_value(backend).await
        }
    }

    // Run compression verification test if compression is enabled
    match compression {
        Compression::Disabled => {
            // No compression to test
        }
        Compression::Gzip { .. } | Compression::Zstd { .. } => {
            test_compression_is_used(backend).await;
        }
    }
}

// ==================== Moka Backend Tests ====================

#[tokio::test]
async fn test_moka_all_combinations() {
    for (key_format, value_format, compression) in TEST_CONFIGS.iter() {
        let config = BackendConfig {
            key: KeyFormat {
                format: key_format.clone(),
            },
            value: ValueFormat {
                format: value_format.clone(),
                compression: compression.clone(),
            },
            backend: Moka { max_capacity: 1000 },
        };

        // Skip configurations with unavailable compression features
        let compressor = match config.value.compression.to_compressor() {
            Ok(c) => c,
            Err(_) => continue, // Skip this combination if compression feature not available
        };

        let backend = MokaBackend::builder(config.backend.max_capacity)
            .key_format(config.key.format.to_cache_key_format())
            .value_format(config.value.format.to_serializer())
            .compressor(compressor)
            .build();

        run_comprehensive_backend_tests(&backend, key_format, value_format, compression).await;
    }
}

// ==================== FeOxDb Backend Tests ====================

#[tokio::test]
async fn test_feoxdb_all_combinations() {
    for (key_format, value_format, compression) in TEST_CONFIGS.iter() {
        let config = BackendConfig {
            key: KeyFormat {
                format: key_format.clone(),
            },
            value: ValueFormat {
                format: value_format.clone(),
                compression: compression.clone(),
            },
            backend: FeOxDb { path: None },
        };

        // Skip configurations with unavailable compression features
        let compressor = match config.value.compression.to_compressor() {
            Ok(c) => c,
            Err(_) => continue, // Skip this combination if compression feature not available
        };

        let backend = FeOxDbBackend::builder()
            .key_format(config.key.format.to_cache_key_format())
            .value_format(config.value.format.to_serializer())
            .compressor(compressor)
            .build()
            .expect("failed to create backend");

        run_comprehensive_backend_tests(&backend, key_format, value_format, compression).await;
    }
}

// ==================== Redis Backend Tests ====================

#[tokio::test]
async fn test_redis_all_combinations() {
    let container: ContainerAsync<RedisContainer> = RedisContainer::default()
        .start()
        .await
        .expect("failed to start Redis container");

    let host = container.get_host().await.expect("failed to get host");
    let host_port = container
        .get_host_port_ipv4(6379)
        .await
        .expect("failed to get port");
    let connection_string = format!("redis://{}:{}", host, host_port);

    for (key_format, value_format, compression) in TEST_CONFIGS.iter() {
        let config = BackendConfig {
            key: KeyFormat {
                format: key_format.clone(),
            },
            value: ValueFormat {
                format: value_format.clone(),
                compression: compression.clone(),
            },
            backend: Redis {
                connection_string: connection_string.clone(),
            },
        };

        // Skip configurations with unavailable compression features
        let compressor = match config.value.compression.to_compressor() {
            Ok(c) => c,
            Err(_) => continue, // Skip this combination if compression feature not available
        };

        let backend = RedisBackend::builder()
            .server(config.backend.connection_string.clone())
            .key_format(config.key.format.to_cache_key_format())
            .value_format(config.value.format.to_serializer())
            .compressor(compressor)
            .build()
            .expect("failed to create backend");

        run_comprehensive_backend_tests(&backend, key_format, value_format, compression).await;
    }
}
