# hitbox-grpc

gRPC predicates and extractors for the [Hitbox](https://github.com/hit-box/hitbox/) caching framework.

Provides gRPC-aware caching primitives that work with the existing `hitbox-tower` middleware:

- **Predicates**: `GrpcService`, `GrpcMethod`, `GrpcStatus`, `GrpcProtoField`
- **Extractors**: `GrpcService`, `GrpcMethod`, `Metadata`, `GrpcProtoField`, `GrpcProtoHash`
- **Utilities**: gRPC status codes, path parsing, frame handling
