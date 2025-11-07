#[cfg(feature = "moka")]
#[test]
fn test_moka_backend_instantiation() {
    use hitbox_configuration::backend::{Backend, BackendConfig, KeyFormat, KeySerialization, Moka, ValueFormat, ValueSerialization, Compression};

    let backend_config = Backend::Moka(BackendConfig {
        key: KeyFormat {
            format: KeySerialization::Bitcode,
        },
        value: ValueFormat {
            format: ValueSerialization::Json,
            compression: Compression::Disabled,
        },
        backend: Moka {
            max_capacity: 1000,
        },
    });

    let _backend = backend_config.into_backend().expect("failed to instantiate backend");

    // If we got here, the backend was successfully instantiated
}

