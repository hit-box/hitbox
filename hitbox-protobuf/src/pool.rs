use prost_reflect::{
    DescriptorPool as ProstPool, MessageDescriptor, MethodDescriptor, ServiceDescriptor,
};

use crate::ProtoError;

/// Pre-loaded protobuf descriptor pool for resolving message types at runtime.
///
/// Wraps `prost_reflect::DescriptorPool` with convenience methods for looking up
/// message descriptors by service/method name. The pool is typically loaded once
/// at startup from a compiled `FileDescriptorSet`.
///
/// # Loading
///
/// The primary path uses pre-compiled `.bin` files (from `prost-build` or `protoc`):
///
/// ```ignore
/// use hitbox_protobuf::DescriptorPool;
///
/// let fds_bytes = include_bytes!("path/to/descriptor.bin");
/// let pool = DescriptorPool::from_file_descriptor_set(fds_bytes)?;
/// ```
///
/// With the `proto_files` feature, `.proto` files can be compiled at runtime:
///
/// ```ignore
/// let pool = DescriptorPool::from_proto_files(
///     &["proto/service.proto"],
///     &["proto/"],
/// )?;
/// ```
#[derive(Debug, Clone)]
pub struct DescriptorPool {
    pool: ProstPool,
}

impl DescriptorPool {
    /// Load from a compiled `FileDescriptorSet` (serialized protobuf bytes).
    ///
    /// This is the primary loading path. Generate the `.bin` file using:
    /// - `prost-build` / `prost-reflect-build` in a `build.rs`
    /// - `protoc --descriptor_set_out=...`
    /// - tonic-build's `FILE_DESCRIPTOR_SET`
    pub fn from_file_descriptor_set(bytes: &[u8]) -> Result<Self, ProtoError> {
        let pool = ProstPool::decode(bytes)?;
        Ok(Self { pool })
    }

    /// Compile `.proto` files at runtime and load their descriptors.
    ///
    /// Requires the `proto_files` feature.
    #[cfg(feature = "proto_files")]
    pub fn from_proto_files(
        proto_files: &[impl AsRef<std::path::Path>],
        includes: &[impl AsRef<std::path::Path>],
    ) -> Result<Self, ProtoError> {
        use prost::Message;

        let fds = protox::compile(proto_files, includes)
            .map_err(|e| ProtoError::ProtoFileError(e.to_string()))?;
        let bytes = fds.encode_to_vec();
        Self::from_file_descriptor_set(&bytes)
    }

    /// Look up a message descriptor by its fully-qualified name.
    ///
    /// The name should include the package prefix, e.g. `"mypackage.MyMessage"`.
    pub fn get_message(&self, full_name: &str) -> Result<MessageDescriptor, ProtoError> {
        self.pool
            .get_message_by_name(full_name)
            .ok_or_else(|| ProtoError::DescriptorNotFound(full_name.to_string()))
    }

    /// Resolve the input (request) message type for a service method.
    pub fn get_input_type(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<MessageDescriptor, ProtoError> {
        let method = self.find_method(service_name, method_name)?;
        Ok(method.input())
    }

    /// Resolve the output (response) message type for a service method.
    pub fn get_output_type(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<MessageDescriptor, ProtoError> {
        let method = self.find_method(service_name, method_name)?;
        Ok(method.output())
    }

    fn find_method(
        &self,
        service_name: &str,
        method_name: &str,
    ) -> Result<MethodDescriptor, ProtoError> {
        let service = self.find_service(service_name)?;
        service
            .methods()
            .find(|m| m.name() == method_name)
            .ok_or_else(|| ProtoError::DescriptorNotFound(format!("{service_name}/{method_name}")))
    }

    fn find_service(&self, service_name: &str) -> Result<ServiceDescriptor, ProtoError> {
        // Try fully-qualified first, then fall back to simple name match
        self.pool
            .services()
            .find(|s| s.full_name() == service_name || s.name() == service_name)
            .ok_or_else(|| ProtoError::DescriptorNotFound(service_name.to_string()))
    }
}
