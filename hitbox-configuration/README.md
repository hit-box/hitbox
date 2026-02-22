# hitbox-configuration

YAML/file-based configuration support for the Hitbox caching framework.

This crate provides types and utilities for defining cache configurations
in configuration files, enabling runtime changes without recompilation.

## Overview

- **Endpoint configuration** - Define caching rules per endpoint
- **Predicate parsing** - Configure request/response predicates from YAML
- **Extractor parsing** - Configure cache key extractors from YAML
- **Backend configuration** - Configure backends from configuration files

## Main Types

- [`Endpoint`] - Cache configuration for a single endpoint
- [`EndpointBuilder`] - Fluent builder for endpoint configuration
- [`ConfigEndpoint`] - Parsed configuration from YAML/files
- [`Backend`] - Backend configuration from files

## Usage

```rust
use std::time::Duration;
use hitbox::policy::PolicyConfig;
use hitbox_configuration::Endpoint;
use hitbox_http::extractors::Method;
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use http_body_util::Full;
use bytes::Bytes;

type Body = Full<Bytes>;

let config = Endpoint::<Body, Body>::builder()
    .request_predicate(NeutralRequestPredicate::new())
    .response_predicate(NeutralResponsePredicate::new())
    .extractor(Method::new())
    .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
    .build();
```

## YAML Configuration Example

```yaml
backend:
  type: Moka
  max_capacity: 10000

request:
  - Method: GET
  - Path: "/api/users/{id}"

response:
  - Status: Success

extractors:
  - Method:
  - Path: "/api/users/{id}"

policy:
  Enabled:
    ttl: 60
    stale: 300
```
