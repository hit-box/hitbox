#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub mod decode;
pub mod descriptor;
pub mod extractors;
pub mod predicates;

// Re-export for internal use by predicates and extractors
pub(crate) use hitbox_http::CacheableSubject;

/// Test utilities shared across modules.
///
/// Provides a lazily-compiled test proto descriptor set to avoid filesystem
/// race conditions when tests run in parallel.
#[cfg(test)]
pub(crate) mod test_util {
    use std::sync::LazyLock;

    use crate::descriptor::ProtoDescriptors;

    /// Pre-compiled test proto descriptor bytes.
    ///
    /// Compiled once on first access, safe for parallel test use.
    static TEST_DESCRIPTOR_BYTES: LazyLock<Vec<u8>> = LazyLock::new(|| {
        let proto_content = r#"
            syntax = "proto3";
            package test;
            message GetUserRequest {
                int64 user_id = 1;
                string name = 2;
            }
            message Ping {
                string msg = 1;
            }
        "#;

        let dir = std::env::temp_dir().join("hitbox_proto_compile_once");
        std::fs::create_dir_all(&dir).unwrap();
        let proto_path = dir.join("test.proto");
        std::fs::write(&proto_path, proto_content).unwrap();

        let fds = protox::compile([&proto_path], [&dir]).unwrap();
        prost::Message::encode_to_vec(&fds)
    });

    /// Returns test descriptors from pre-compiled bytes (thread-safe).
    pub fn test_descriptors() -> ProtoDescriptors {
        ProtoDescriptors::from_bytes(&TEST_DESCRIPTOR_BYTES).unwrap()
    }
}
