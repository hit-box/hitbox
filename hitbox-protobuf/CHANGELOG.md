# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-03-07

### Added
- `DescriptorPool` for loading protobuf descriptors from `FileDescriptorSet` bytes or `.proto` files (behind `proto_files` feature)
- `ProtoFields` predicate for checking protobuf field values against operations (`Eq`, `NotEq`, `Gt`, `Lt`, `Contains`, `Regex`, `In`, `Exists`, `Path`)
- `ProtoFieldsExtractor` for extracting protobuf field values as cache key parts
- `FieldsBuilder` for ergonomic batch field specification
- `FrameDecoder` trait with `NoFraming` implementation for raw protobuf bodies
- `ProtoValue` enum mapping protobuf scalar types for comparison operations
- Dotted path support for nested message field access (e.g., `"address.city"`)
