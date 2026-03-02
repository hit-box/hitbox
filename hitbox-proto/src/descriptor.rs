//! Protobuf descriptor loading utilities.
//!
//! Provides [`ProtoDescriptors`] for loading and querying protobuf message descriptors,
//! used by predicates and extractors to inspect protobuf message fields at runtime.
//!
//! # Examples
//!
//! ```no_run
//! use hitbox_proto::descriptor::ProtoDescriptors;
//!
//! // Load from compiled FileDescriptorSet bytes
//! let bytes = std::fs::read("descriptors.bin").unwrap();
//! let descriptors = ProtoDescriptors::from_bytes(&bytes).unwrap();
//!
//! // Look up a message by fully-qualified name
//! let msg_desc = descriptors.get_message("my.package.GetUserRequest").unwrap();
//! ```

use std::path::Path;
use std::sync::Arc;

use prost_reflect::{DescriptorPool, MessageDescriptor};

/// Shared protobuf descriptor pool for looking up message types at runtime.
///
/// Wraps a [`DescriptorPool`] in an `Arc` so it can be cheaply cloned and shared
/// across predicates and extractors.
///
/// # Construction
///
/// - [`from_bytes`](Self::from_bytes) — load a pre-compiled `FileDescriptorSet` (fastest, recommended for production)
/// - [`from_proto_files`](Self::from_proto_files) — compile `.proto` files at runtime using `protox` (convenient for development/testing)
#[derive(Debug, Clone)]
pub struct ProtoDescriptors {
    pool: Arc<DescriptorPool>,
}

impl ProtoDescriptors {
    /// Creates descriptors from a compiled `FileDescriptorSet` encoded as bytes.
    ///
    /// This is the recommended approach for production use. Generate the descriptor
    /// set at build time using `protoc --descriptor_set_out=...` or `prost-build`.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not a valid `FileDescriptorSet`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DescriptorError> {
        let pool = DescriptorPool::decode(bytes).map_err(DescriptorError::Decode)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Compiles `.proto` files at runtime and creates descriptors.
    ///
    /// Useful for development, testing, or when proto files are available but
    /// a compiled descriptor set is not.
    ///
    /// # Arguments
    ///
    /// * `includes` — directories to search for imports (like `protoc -I`)
    /// * `files` — `.proto` files to compile
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails (syntax errors, missing imports, etc.).
    pub fn from_proto_files<I, F>(includes: I, files: F) -> Result<Self, DescriptorError>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
        F: IntoIterator,
        F::Item: AsRef<Path>,
    {
        let file_descriptor_set =
            protox::compile(files, includes).map_err(DescriptorError::Compile)?;
        let pool = DescriptorPool::from_file_descriptor_set(file_descriptor_set)
            .map_err(DescriptorError::Decode)?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Looks up a message descriptor by fully-qualified name.
    ///
    /// The name should include the package, e.g. `"my.package.GetUserRequest"`.
    ///
    /// Returns `None` if no message with that name exists in the descriptor pool.
    pub fn get_message(&self, name: &str) -> Option<MessageDescriptor> {
        self.pool.get_message_by_name(name)
    }

    /// Returns a reference to the underlying [`DescriptorPool`].
    pub fn pool(&self) -> &DescriptorPool {
        &self.pool
    }
}

/// Errors that can occur when loading protobuf descriptors.
#[derive(Debug)]
pub enum DescriptorError {
    /// Failed to decode a `FileDescriptorSet` from bytes.
    Decode(prost_reflect::DescriptorError),
    /// Failed to compile `.proto` files.
    Compile(protox::Error),
}

impl std::fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DescriptorError::Decode(e) => write!(f, "failed to decode descriptor set: {e}"),
            DescriptorError::Compile(e) => write!(f, "failed to compile proto files: {e}"),
        }
    }
}

impl std::error::Error for DescriptorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DescriptorError::Decode(e) => Some(e),
            DescriptorError::Compile(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_and_lookup_message() {
        let descriptors = crate::test_util::test_descriptors();
        let msg = descriptors.get_message("test.GetUserRequest");
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.name(), "GetUserRequest");
        assert_eq!(msg.full_name(), "test.GetUserRequest");
    }

    #[test]
    fn lookup_nonexistent_message() {
        let descriptors = crate::test_util::test_descriptors();
        assert!(descriptors.get_message("test.NonExistent").is_none());
    }

    #[test]
    fn message_has_expected_fields() {
        let descriptors = crate::test_util::test_descriptors();
        let msg = descriptors.get_message("test.GetUserRequest").unwrap();
        assert!(msg.get_field_by_name("user_id").is_some());
        assert!(msg.get_field_by_name("name").is_some());
        assert!(msg.get_field_by_name("nonexistent").is_none());
    }

    #[test]
    fn from_bytes_round_trip() {
        // The shared test fixture already exercises from_bytes() internally.
        // Verify it works and contains the expected messages.
        let descriptors = crate::test_util::test_descriptors();
        assert!(descriptors.get_message("test.GetUserRequest").is_some());
        assert!(descriptors.get_message("test.Ping").is_some());
    }
}
