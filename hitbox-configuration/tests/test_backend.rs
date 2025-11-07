use hitbox_configuration::backend::{Backend, BackendConfig, Compression, KeyFormat, KeySerialization, Moka, ValueFormat, ValueSerialization};

#[test]
fn test_moka_backend_deserialize() {
    let yaml = r#"
type: Moka
max_capacity: 10000
key:
  format: Bitcode
value:
  format: Json
  compression: Zstd
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Moka(config) => {
            assert_eq!(config.backend.max_capacity, 10000);
            assert_eq!(config.key.format, KeySerialization::Bitcode);
            assert_eq!(config.value.format, ValueSerialization::Json);
            assert_eq!(config.value.compression, Compression::Zstd);
        }
        _ => panic!("expected Moka backend"),
    }
}

#[test]
fn test_feoxdb_backend_deserialize() {
    let yaml = r#"
type: FeOxDb
path: "/tmp/cache.db"
key:
  format: UrlEncoded
value:
  format: Bincode
  compression: Zstd
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::FeOxDb(config) => {
            assert_eq!(config.backend.path, Some("/tmp/cache.db".to_string()));
            assert_eq!(config.key.format, KeySerialization::UrlEncoded);
            assert_eq!(config.value.format, ValueSerialization::Bincode);
            assert_eq!(config.value.compression, Compression::Zstd);
        }
        _ => panic!("expected FeOxDb backend"),
    }
}

#[test]
fn test_redis_backend_deserialize() {
    let yaml = r#"
type: Redis
connection_string: "redis://localhost:6379"
key:
  format: Bitcode
value:
  format: Json
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Redis(config) => {
            assert_eq!(config.backend.connection_string, "redis://localhost:6379");
            assert_eq!(config.key.format, KeySerialization::Bitcode);
            assert_eq!(config.value.format, ValueSerialization::Json);
            assert_eq!(config.value.compression, Compression::Disabled);
        }
        _ => panic!("expected Redis backend"),
    }
}

#[test]
fn test_backend_serialize_roundtrip() {
    let backend = Backend::Moka(BackendConfig {
        key: KeyFormat {
            format: KeySerialization::Bitcode,
        },
        value: ValueFormat {
            format: ValueSerialization::Json,
            compression: Compression::Zstd,
        },
        backend: Moka {
            max_capacity: 5000,
        },
    });

    let yaml = serde_saphyr::to_string(&backend).expect("failed to serialize");
    println!("Serialized YAML:\n{}", yaml);
    let deserialized: Backend = serde_saphyr::from_str(&yaml).expect("failed to deserialize");

    assert_eq!(backend, deserialized);
}
