//! The cacheable derives must generate paths through the `hitbox` facade only.
//!
//! This crate depends on `hitbox` and `hitbox-derive` but not on `hitbox-core`,
//! like a typical consumer, so any generated `hitbox_core::` path fails to
//! compile here.

use hitbox_derive::{CacheableRequest, CacheableResponse};
use serde::{Deserialize, Serialize};

#[derive(CacheableRequest)]
#[allow(dead_code)]
struct SearchRequest {
    query: String,
    page: u32,
}

#[derive(Clone, Serialize, Deserialize, CacheableResponse)]
#[allow(dead_code)]
struct User {
    id: u64,
    name: String,
}

#[test]
fn derives_compile_without_direct_hitbox_core_dependency() {}
