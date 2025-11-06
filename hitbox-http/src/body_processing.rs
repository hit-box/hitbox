use std::fmt::Debug;

use bytes::Bytes;
use hyper::body::Body as HttpBody;
use jaq_core::{
    Ctx, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::{self, Val};
use prost_reflect::{DynamicMessage, MessageDescriptor, SerializeOptions};
use serde_json::Value;

/// Maximum body size for buffering (10KB)
pub const MAX_BODY_SIZE: usize = 10 * 1024;

/// Error that occurs during body collection
#[derive(Debug)]
pub enum BodyCollectionError {
    /// Failed to collect body from stream
    CollectionFailed,
    /// Body size exceeds maximum allowed size
    SizeExceeded { actual: usize, max: usize },
}

/// Error that occurs during body parsing
#[derive(Debug)]
pub enum BodyParsingError {
    /// Failed to parse JSON
    JsonParsingFailed,
    /// Failed to decode ProtoBuf
    ProtoBufDecodeFailed,
    /// Failed to serialize ProtoBuf to JSON
    ProtoBufSerializeFailed,
}

/// Error that occurs during JQ expression evaluation
#[derive(Debug)]
pub enum JqError {
    /// Failed to load JQ modules
    ModuleLoadFailed,
    /// Failed to compile JQ expression
    CompilationFailed,
}

/// Body parsing type
#[derive(Debug, Clone)]
pub enum ParsingType {
    Jq,
    ProtoBuf(MessageDescriptor),
}

/// Collect body from HTTP stream with size limit
///
/// This implementation manually checks the size limit during collection rather than
/// using `Limited` wrapper, to avoid complex trait bound requirements that would
/// need to propagate through the entire codebase.
pub async fn collect_body<B>(body: B, max_size: usize) -> Result<Bytes, BodyCollectionError>
where
    B: HttpBody,
    B::Error: Debug,
    B::Data: Send + bytes::Buf,
{
    use bytes::{Buf, BufMut};
    use http_body_util::BodyExt;
    use std::pin::Pin;

    let mut body = Box::pin(body);
    let mut collected = Vec::new();
    let mut total_size = 0;

    // Collect frame by frame, checking size limit as we go
    while let Some(frame) = Pin::as_mut(&mut body)
        .frame()
        .await
        .transpose()
        .map_err(|_| BodyCollectionError::CollectionFailed)?
    {
        if let Ok(data) = frame.into_data() {
            total_size += data.remaining();

            // Check limit BEFORE collecting data
            if total_size > max_size {
                return Err(BodyCollectionError::SizeExceeded {
                    actual: total_size,
                    max: max_size,
                });
            }

            collected.put(data);
        }
    }

    Ok(Bytes::from(collected))
}

/// Parse body bytes into JSON value based on parsing type
pub fn parse_body(bytes: &[u8], parsing_type: &ParsingType) -> Result<Value, BodyParsingError> {
    match parsing_type {
        ParsingType::Jq => {
            let body_str = String::from_utf8_lossy(bytes);
            serde_json::from_str(&body_str).or(Ok(Value::Null))
        }
        ParsingType::ProtoBuf(message) => {
            let dynamic_message = DynamicMessage::decode(message.clone(), bytes)
                .map_err(|_| BodyParsingError::ProtoBufDecodeFailed)?;

            let mut serializer = serde_json::Serializer::new(vec![]);
            let options = SerializeOptions::new().skip_default_fields(false);

            dynamic_message
                .serialize_with_options(&mut serializer, &options)
                .map_err(|_| BodyParsingError::ProtoBufSerializeFailed)?;

            serde_json::from_slice(&serializer.into_inner())
                .map_err(|_| BodyParsingError::ProtoBufSerializeFailed)
        }
    }
}

/// Apply JQ expression to JSON value
pub fn apply_jq_expression(expression: &str, input: Value) -> Result<Option<Value>, JqError> {
    let program = File {
        code: expression,
        path: (),
    };

    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();

    let modules = loader
        .load(&arena, program)
        .map_err(|_| JqError::ModuleLoadFailed)?;

    let filter = jaq_core::Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|_| JqError::CompilationFailed)?;

    let inputs = RcIter::new(core::iter::empty());
    let out = filter.run((Ctx::new([], &inputs), Val::from(input)));
    let results: Result<Vec<_>, _> = out.collect();

    match results {
        Ok(values) if values.eq(&vec![Val::Null]) => Ok(None),
        Ok(values) if !values.is_empty() => {
            let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
            if values.len() == 1 {
                Ok(Some(values.into_iter().next().unwrap()))
            } else {
                Ok(Some(Value::Array(values)))
            }
        }
        _ => Ok(None),
    }
}
