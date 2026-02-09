//! Memoization with Derive Macros
//!
//! Demonstrates all hitbox derive macros:
//! - `#[derive(KeyExtract)]` - Custom types contribute to cache keys
//! - `#[derive(CacheableResponse)]` - Response types with skip attribute
//! - `#[cached]` - Transform async functions into cacheable functions
//!
//! Run:
//!   cargo run -p hitbox-examples --example memoization_derive

use std::time::Duration;

use hitbox::CacheStatus;
use hitbox_fn::Cache;
use hitbox_fn::prelude::*;
use hitbox_moka::MokaBackend;
use serde::{Deserialize, Serialize};

// =============================================================================
// Domain Types with KeyExtract
// =============================================================================

#[derive(Debug, Clone, Copy, KeyExtract)]
pub struct UserId(#[key_extract(name = "user_id")] pub u64);

#[derive(Debug, Clone, Copy, KeyExtract)]
pub struct OrgId(#[key_extract(name = "org_id")] pub u64);

#[derive(Debug, Clone, KeyExtract)]
pub struct SearchQuery {
    #[key_extract(name = "q")]
    pub query: String,
    pub page: u32,
    #[key_extract(skip)]
    pub request_id: String,
}

// =============================================================================
// Response Types with CacheableResponse
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CacheableResponse)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct UserProfile {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CacheableResponse)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct AuthResult {
    pub user_id: u64,
    pub permissions: Vec<String>,
    #[cacheable_response(skip)]
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApiError;

// =============================================================================
// Cached Functions
// =============================================================================

#[cached]
pub async fn get_user(user_id: UserId) -> Result<UserProfile, ApiError> {
    println!("[upstream] get_user({})", user_id.0);
    Ok(UserProfile {
        id: user_id.0,
        name: format!("User {}", user_id.0),
    })
}

#[cached(prefix = "org_user")]
pub async fn get_org_user(org_id: OrgId, user_id: UserId) -> Result<UserProfile, ApiError> {
    println!("[upstream] get_org_user({}, {})", org_id.0, user_id.0);
    Ok(UserProfile {
        id: user_id.0,
        name: format!("User {} @ Org {}", user_id.0, org_id.0),
    })
}

#[cached(prefix = "auth")]
pub async fn authenticate(user_id: UserId) -> Result<AuthResult, ApiError> {
    println!("[upstream] authenticate({})", user_id.0);
    Ok(AuthResult {
        user_id: user_id.0,
        permissions: vec!["read".into(), "write".into()],
        access_token: Some("secret-token".into()),
    })
}

// Note: Vec<String> and String have blanket CacheableResponse impls
#[cached(prefix = "search")]
pub async fn search(query: SearchQuery) -> Result<Vec<String>, ApiError> {
    println!("[upstream] search(q='{}')", query.query);
    Ok(vec![format!("Result for '{}'", query.query)])
}

#[cached(prefix = "config")]
pub async fn get_config() -> Result<String, ApiError> {
    println!("[upstream] get_config()");
    Ok("config_v1".into())
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() {
    let cache = Cache::builder()
        .backend(MokaBackend::builder().max_entries(100).build())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    // 1. Basic memoization
    let (r1, c1) = get_user(UserId(1)).cache(&cache).with_context().await;
    let (r2, c2) = get_user(UserId(1)).cache(&cache).with_context().await;
    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);

    // 2. Multiple args
    let (_, c1) = get_org_user(OrgId(1), UserId(1))
        .cache(&cache)
        .with_context()
        .await;
    let (_, c2) = get_org_user(OrgId(1), UserId(1))
        .cache(&cache)
        .with_context()
        .await;
    let (_, c3) = get_org_user(OrgId(2), UserId(1))
        .cache(&cache)
        .with_context()
        .await;
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
    assert_eq!(c3.status, CacheStatus::Miss); // Different org = different key

    // 3. Skip in CacheableResponse (tokens not cached but returned on miss)
    let (r1, c1) = authenticate(UserId(1)).cache(&cache).with_context().await;
    let (r2, c2) = authenticate(UserId(1)).cache(&cache).with_context().await;
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
    assert!(r1.as_ref().unwrap().access_token.is_some()); // Present on miss
    assert!(r2.as_ref().unwrap().access_token.is_none()); // Skipped on hit (not in cache)

    // 4. Skip in KeyExtract (request_id not in cache key)
    let q1 = SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-1".into(),
    };
    let q2 = SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-2".into(),
    };
    let (_, c1) = search(q1).cache(&cache).with_context().await;
    let (_, c2) = search(q2).cache(&cache).with_context().await; // Same key despite different request_id
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);

    // 5. Zero-argument function
    let (_, c1) = get_config().cache(&cache).with_context().await;
    let (_, c2) = get_config().cache(&cache).with_context().await;
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}
