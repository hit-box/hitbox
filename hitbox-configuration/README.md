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
use bytes::Bytes;
use http_body_util::Empty;
use hitbox::policy::PolicyConfig;
use hitbox_configuration::Endpoint;
use hitbox_http::extractors::Method;
use hitbox_http::predicates::{request, response};

let config = Endpoint::<Empty<Bytes>, Empty<Bytes>>::builder()
    .request_predicate(request::predicate())
    .response_predicate(response::predicate())
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
