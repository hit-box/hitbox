#![allow(dead_code)]

use bytes::Bytes;
use http_body_util::Empty;
use prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, Value};

use hitbox_http::{BufferedBody, CacheableHttpRequest};
use hitbox_protobuf::DescriptorPool;

/// Load the test descriptor pool compiled by build.rs.
pub fn test_pool() -> DescriptorPool {
    let bytes = include_bytes!(env!("TEST_DESCRIPTOR_PATH"));
    DescriptorPool::from_file_descriptor_set(bytes).expect("failed to load test descriptors")
}

/// Build a `CacheableHttpRequest` from raw bytes.
pub fn request_from_bytes(
    method: &str,
    uri: &str,
    body_bytes: Bytes,
) -> CacheableHttpRequest<Empty<Bytes>> {
    let body = BufferedBody::Complete {
        data: Some(body_bytes),
        trailers: None,
    };
    let request = http::Request::builder()
        .method(method)
        .uri(uri)
        .body(body)
        .unwrap();
    CacheableHttpRequest::from_request(request)
}

/// Fluent builder for test protobuf messages.
pub struct TestRequest {
    message_name: String,
    fields: Vec<(String, Value)>,
}

impl TestRequest {
    pub fn new(message_name: &str) -> Self {
        Self {
            message_name: message_name.to_string(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: &str, value: Value) -> Self {
        self.fields.push((name.to_string(), value));
        self
    }

    pub fn message_descriptor(&self) -> prost_reflect::MessageDescriptor {
        test_pool().get_message(&self.message_name).unwrap()
    }

    pub fn build_message(&self) -> DynamicMessage {
        let desc = self.message_descriptor();
        let mut msg = DynamicMessage::new(desc);
        for (name, value) in &self.fields {
            let fd = msg.descriptor().get_field_by_name(name).unwrap();
            msg.set_field(&fd, value.clone());
        }
        msg
    }

    pub fn build_bytes(&self) -> Bytes {
        Bytes::from(self.build_message().encode_to_vec())
    }

    pub fn build_request(
        &self,
        method: &str,
        uri: &str,
    ) -> (
        prost_reflect::MessageDescriptor,
        CacheableHttpRequest<Empty<Bytes>>,
    ) {
        (
            self.message_descriptor(),
            request_from_bytes(method, uri, self.build_bytes()),
        )
    }
}
