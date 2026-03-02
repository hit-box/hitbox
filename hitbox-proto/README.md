# hitbox-proto

Protobuf predicates and extractors for the [Hitbox](https://github.com/hit-box/hitbox/) caching framework.

Provides shared protobuf inspection utilities used by `hitbox-grpc` and `hitbox-twirp`:

- **Descriptor loading** — load protobuf `FileDescriptorSet` from compiled bytes or `.proto` files
- **Dynamic decoding** — decode protobuf messages at runtime using `prost-reflect`
- **Typed decoding** — decode protobuf messages at compile time using `prost::Message`
- **Predicates** — `ProtoField` (runtime) and `TypedProtoField` (compile-time) for field-level cache decisions
- **Extractors** — `ProtoFieldExtractor`, `TypedProtoExtractor`, and `ProtoHash` for protobuf-aware cache keys
