use hitbox_backend::format::FormatTypeId;
use hitbox_configuration::backend::{
    Backend, BackendConfig, Compression, KeyFormat, KeySerialization, Moka, ReadPolicy,
    RefillPolicyConfig, ValueFormat, ValueSerialization, WritePolicy,
};

#[test]
fn test_moka_backend_deserialize() {
    let yaml = r#"
type: Moka
max_capacity: 10000
key:
  format: Bitcode
value:
  format: Json
  compression:
    type: Zstd
    level: 3
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Moka(config) => {
            assert_eq!(config.backend.max_capacity, 10000);
            assert_eq!(config.key.format, KeySerialization::Bitcode);
            assert_eq!(config.value.format, ValueSerialization::Json);
            assert_eq!(config.value.compression, Compression::Zstd { level: 3 });
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
  compression:
    type: Zstd
    level: 3
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::FeOxDb(config) => {
            assert_eq!(config.backend.path, Some("/tmp/cache.db".to_string()));
            assert_eq!(config.key.format, KeySerialization::UrlEncoded);
            assert_eq!(config.value.format, ValueSerialization::Bincode);
            assert_eq!(config.value.compression, Compression::Zstd { level: 3 });
        }
        _ => panic!("expected FeOxDb backend"),
    }
}

#[test]
fn test_postcard_value_format() {
    let yaml = r#"
type: Moka
max_capacity: 100
key:
  format: Bitcode
value:
  format: Postcard
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    let Backend::Moka(config) = backend else {
        panic!("expected Moka backend");
    };
    assert_eq!(config.value.format, ValueSerialization::Postcard);
    assert_eq!(
        config.value.format.to_serializer().format_type_id(),
        FormatTypeId::Postcard
    );
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
            compression: Compression::Zstd { level: 3 },
        },
        backend: Moka {
            max_capacity: 5000,
            label: None,
        },
    });

    let yaml = serde_saphyr::to_string(&backend).expect("failed to serialize");
    println!("Serialized YAML:\n{}", yaml);
    let deserialized: Backend = serde_saphyr::from_str(&yaml).expect("failed to deserialize");

    assert_eq!(backend, deserialized);
}

#[test]
fn test_backend_with_custom_label() {
    let yaml = r#"
type: Moka
max_capacity: 10000
label: "session-cache"
key:
  format: Bitcode
value:
  format: Json
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Moka(config) => {
            assert_eq!(config.backend.max_capacity, 10000);
            assert_eq!(config.backend.label, Some("session-cache".to_string()));
        }
        _ => panic!("expected Moka backend"),
    }
}

#[test]
fn test_composition_backend_with_labeled_layers() {
    let yaml = r#"
type: Composition
label: "tiered-cache"
l1:
  type: Moka
  max_capacity: 1000
  label: "l1-hot-cache"
  key:
    format: Bitcode
  value:
    format: Json
l2:
  type: Redis
  connection_string: "redis://localhost:6379"
  label: "l2-persistent"
  key:
    format: Bitcode
  value:
    format: Json
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Composition(config) => {
            assert_eq!(config.label, Some("tiered-cache".to_string()));
            match config.l1.as_ref() {
                Backend::Moka(moka) => {
                    assert_eq!(moka.backend.label, Some("l1-hot-cache".to_string()));
                }
                _ => panic!("expected Moka as L1"),
            }
            match config.l2.as_ref() {
                Backend::Redis(redis) => {
                    assert_eq!(redis.backend.label, Some("l2-persistent".to_string()));
                }
                _ => panic!("expected Redis as L2"),
            }
        }
        _ => panic!("expected Composition backend"),
    }
}

#[test]
fn test_composition_backend_with_default_policies() {
    let yaml = r#"
type: Composition
l1:
  type: Moka
  max_capacity: 5000
  key:
    format: Bitcode
  value:
    format: Json
l2:
  type: FeOxDb
  path: "/tmp/cache.db"
  key:
    format: UrlEncoded
  value:
    format: Bincode
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Composition(config) => {
            // Check default policies
            assert_eq!(config.policy.read, ReadPolicy::Sequential);
            assert_eq!(config.policy.write, WritePolicy::OptimisticParallel);
            assert_eq!(config.policy.refill, RefillPolicyConfig::Never);
        }
        _ => panic!("expected Composition backend"),
    }
}

#[test]
fn test_nested_composition_backend() {
    let yaml = r#"
type: Composition
l1:
  type: Moka
  max_capacity: 1000
  key:
    format: Bitcode
  value:
    format: Bincode
l2:
  type: Composition
  l1:
    type: Moka
    max_capacity: 10000
    key:
      format: Bitcode
    value:
      format: Bincode
  l2:
    type: Redis
    connection_string: "redis://localhost:6379"
    key:
      format: Bitcode
    value:
      format: Bincode
  policy:
    read: Sequential
    write: OptimisticParallel
    refill: Never
policy:
  read: Race
  write: OptimisticParallel
  refill: Always
"#;

    let backend: Backend = serde_saphyr::from_str(yaml).expect("failed to deserialize");

    match backend {
        Backend::Composition(config) => {
            // Check outer policies
            assert_eq!(config.policy.read, ReadPolicy::Race);
            assert_eq!(config.policy.refill, RefillPolicyConfig::Always);

            // Check L1 is Moka
            assert!(matches!(config.l1.as_ref(), Backend::Moka(_)));

            // Check L2 is Composition
            match config.l2.as_ref() {
                Backend::Composition(inner) => {
                    assert!(matches!(inner.l1.as_ref(), Backend::Moka(_)));
                    assert!(matches!(inner.l2.as_ref(), Backend::Redis(_)));
                    assert_eq!(inner.policy.read, ReadPolicy::Sequential);
                }
                _ => panic!("expected nested Composition as L2"),
            }
        }
        _ => panic!("expected Composition backend"),
    }
}
