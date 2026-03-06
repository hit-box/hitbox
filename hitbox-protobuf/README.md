# hitbox-protobuf

Protobuf field predicates and extractors for the [Hitbox](https://github.com/hit-box/hitbox) caching framework.

This crate provides protocol-agnostic protobuf message inspection for cache
decisions and key generation. It uses [`prost-reflect`](https://docs.rs/prost-reflect)
for dynamic message decoding — no generated code required at runtime.

## Core Concepts

- **[`DescriptorPool`]**: Loads protobuf descriptors from compiled
  `FileDescriptorSet` bytes or (with the `proto_files` feature) from `.proto`
  source files at runtime.

- **[`ProtoFields`]** predicate: Decodes a protobuf body and checks field values
  against [`Operation`]s (equality, range, regex, containment, etc.).
  Decode happens once per request; multiple fields are checked in a single pass.

- **[`ProtoFieldsExtractor`]**: Extracts protobuf field values as cache key parts.

- **[`FrameDecoder`]** trait: Strips protocol-specific framing before protobuf
  decode. Ships with [`NoFraming`] (identity, for Twirp / raw protobuf);
  `hitbox-grpc` provides the gRPC 5-byte length-prefix decoder.

## Quickstart

```rust,ignore
use hitbox_protobuf::{
    DescriptorPool, FieldsBuilder, Operation, ProtoValue,
    ProtoFieldsPredicate, ProtoFieldsExtract,
};

// 1. Load descriptors (typically from a build.rs-generated .bin)
let pool = DescriptorPool::from_file_descriptor_set(
    include_bytes!("path/to/descriptor.bin"),
)?;
let descriptor = pool.get_message("mypackage.GetUserRequest")?;

// 2. Predicate: only cache when user_id exists and role == "admin"
let fields = FieldsBuilder::new()
    .field("user_id", Operation::Exists)
    .field("role", Operation::Eq(ProtoValue::String("admin".into())))
    .build();

let predicate = request_predicate
    .proto_fields(descriptor.clone(), fields);

// 3. Extractor: include user_id in the cache key
let extractor = base_extractor
    .proto_fields(descriptor, vec!["user_id".into()]);
```

## Operations

Operations are evaluated against protobuf field values after decoding:

| Operation | Description |
|-----------|-------------|
| `Exists` | Field has a non-default value |
| `Eq(value)` | Field equals the given value |
| `NotEq(value)` | Field does not equal the given value |
| `Gt(value)` | Greater than (numeric types) |
| `Lt(value)` | Less than (numeric types) |
| `Contains(substring)` | String/bytes field contains the substring |
| `Regex(pattern)` | String field matches the regex |
| `In(variants)` | Field value is one of the listed values |
| `Path(field, op)` | Traverse into a nested message, then apply `op` |

## Descriptor Loading

### Pre-compiled (recommended)

Generate a `FileDescriptorSet` in your `build.rs` using `prost-build`,
`prost-reflect-build`, `tonic-build`, or `protoc --descriptor_set_out`:

```rust,ignore
let pool = DescriptorPool::from_file_descriptor_set(
    include_bytes!(concat!(env!("OUT_DIR"), "/descriptor.bin")),
)?;
```

### Runtime compilation (optional)

Enable the `proto_files` feature to compile `.proto` files at runtime
using [`protox`](https://docs.rs/protox):

```rust,ignore
let pool = DescriptorPool::from_proto_files(
    &["proto/service.proto"],
    &["proto/"],
)?;
```

## Custom Frame Decoders

Implement [`FrameDecoder`] to support protocols with body framing:

```rust,ignore
use std::borrow::Cow;
use hitbox_protobuf::{FrameDecoder, ProtoError};

struct GrpcFrameDecoder;

impl FrameDecoder for GrpcFrameDecoder {
    fn decode<'a>(&self, body: &'a [u8]) -> Result<Cow<'a, [u8]>, ProtoError> {
        // Strip the 5-byte gRPC length-prefix frame
        if body.len() < 5 { return Err(ProtoError::FrameError("too short".into())); }
        Ok(Cow::Borrowed(&body[5..]))
    }
}
```

Then use it with predicates or extractors:

```rust,ignore
let predicate = base.proto_fields_with_decoder(descriptor, fields, GrpcFrameDecoder);
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `proto_files` | Enables runtime `.proto` compilation via `protox` |

[`DescriptorPool`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/struct.DescriptorPool.html
[`ProtoFields`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/struct.ProtoFields.html
[`ProtoFieldsExtractor`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/struct.ProtoFieldsExtractor.html
[`Operation`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/enum.Operation.html
[`FrameDecoder`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/trait.FrameDecoder.html
[`NoFraming`]: https://docs.rs/hitbox-protobuf/latest/hitbox_protobuf/struct.NoFraming.html
