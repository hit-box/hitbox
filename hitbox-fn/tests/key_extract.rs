//! Integration tests for KeyExtract derive macro.

use hitbox::Extractor;
use hitbox_derive::KeyExtract;
use hitbox_fn::prelude::*;

// =============================================================================
// Test structs with KeyExtract derive
// =============================================================================

#[derive(KeyExtract)]
struct UserId(u64);

#[derive(KeyExtract)]
struct TenantId(#[key_extract(name = "tenant")] String);

#[derive(KeyExtract)]
struct SearchQuery {
    query: String,
    page: u32,
    #[key_extract(skip)]
    #[allow(dead_code)]
    request_id: String,
}

#[derive(KeyExtract)]
struct CustomNamed {
    #[key_extract(name = "user")]
    user_id: u64,
    #[key_extract(name = "org")]
    org_id: u64,
}

#[derive(KeyExtract)]
struct AllSkipped {
    #[key_extract(skip)]
    #[allow(dead_code)]
    internal: String,
}

// =============================================================================
// KeyExtract trait tests
// =============================================================================

#[test]
fn test_newtype_struct() {
    let user = UserId(42);
    let parts = user.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "0"); // Default name for tuple field
    assert_eq!(parts[0].value(), Some("42"));
}

#[test]
fn test_newtype_with_custom_name() {
    let tenant = TenantId("acme".into());
    let parts = tenant.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "tenant");
    assert_eq!(parts[0].value(), Some("acme"));
}

#[test]
fn test_struct_with_skip() {
    let q1 = SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-1".into(),
    };
    let q2 = SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-2".into(), // Different request_id
    };

    let parts1 = q1.extract();
    let parts2 = q2.extract();

    // request_id should be skipped
    assert_eq!(parts1.len(), 2);
    assert_eq!(parts2.len(), 2);

    // Same key parts despite different request_id
    assert_eq!(parts1[0].key(), parts2[0].key());
    assert_eq!(parts1[0].value(), parts2[0].value());
    assert_eq!(parts1[1].key(), parts2[1].key());
    assert_eq!(parts1[1].value(), parts2[1].value());
}

#[test]
fn test_custom_field_names() {
    let custom = CustomNamed {
        user_id: 1,
        org_id: 2,
    };
    let parts = custom.extract();

    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].key(), "user");
    assert_eq!(parts[0].value(), Some("1"));
    assert_eq!(parts[1].key(), "org");
    assert_eq!(parts[1].value(), Some("2"));
}

#[test]
fn test_all_fields_skipped() {
    let skipped = AllSkipped {
        internal: "secret".into(),
    };
    let parts = skipped.extract();

    // When all fields are skipped, should return struct name marker
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "AllSkipped");
    assert_eq!(parts[0].value(), None);
}

// =============================================================================
// FnExtractor integration tests
// =============================================================================

#[tokio::test]
async fn test_extractor_with_derived_type() {
    let extractor = FnExtractor::new("test::get_user");
    let args = Args((UserId(42),));

    let key_parts = extractor.get(args, &hitbox::EvalContext::new()).await;
    let (_, key) = key_parts.into_cache_key();

    let key_str = key.to_string();
    assert!(key_str.contains("test::get_user"));
    assert!(key_str.contains("42"));
}

#[tokio::test]
async fn test_extractor_with_multiple_derived_types() {
    let extractor = FnExtractor::new("test::get_user_for_tenant");
    let args = Args((UserId(1), TenantId("acme".into())));

    let key_parts = extractor.get(args, &hitbox::EvalContext::new()).await;
    let (_, key) = key_parts.into_cache_key();

    let key_str = key.to_string();
    assert!(key_str.contains("test::get_user_for_tenant"));
    assert!(key_str.contains("1"));
    assert!(key_str.contains("acme"));
}

#[tokio::test]
async fn test_extractor_skip_not_affect_key() {
    let extractor = FnExtractor::new("test::search");

    let args1 = Args((SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-111".into(),
    },));
    let args2 = Args((SearchQuery {
        query: "rust".into(),
        page: 1,
        request_id: "req-222".into(),
    },));

    let (_, key1) = extractor
        .get(args1, &hitbox::EvalContext::new())
        .await
        .into_cache_key();
    let (_, key2) = extractor
        .get(args2, &hitbox::EvalContext::new())
        .await
        .into_cache_key();

    // Keys should be equal despite different request_id
    assert_eq!(key1.to_string(), key2.to_string());
}

#[tokio::test]
async fn test_extractor_different_values_different_keys() {
    let extractor = FnExtractor::new("test::get_user");

    let (_, key1) = extractor
        .get(Args((UserId(1),)), &hitbox::EvalContext::new())
        .await
        .into_cache_key();
    let (_, key2) = extractor
        .get(Args((UserId(2),)), &hitbox::EvalContext::new())
        .await
        .into_cache_key();

    assert_ne!(key1.to_string(), key2.to_string());
}

// =============================================================================
// Scalar type tests
// =============================================================================

#[test]
fn test_scalar_u64() {
    let value: u64 = 42;
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "u64");
    assert_eq!(parts[0].value(), Some("42"));
}

#[test]
fn test_scalar_i32() {
    let value: i32 = -123;
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "i32");
    assert_eq!(parts[0].value(), Some("-123"));
}

#[test]
fn test_scalar_string() {
    let value = String::from("hello");
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "str");
    assert_eq!(parts[0].value(), Some("hello"));
}

#[test]
fn test_scalar_str() {
    let value = "world";
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "str");
    assert_eq!(parts[0].value(), Some("world"));
}

#[test]
fn test_scalar_bool() {
    let parts_true = true.extract();
    let parts_false = false.extract();

    assert_eq!(parts_true[0].key(), "bool");
    assert_eq!(parts_true[0].value(), Some("true"));
    assert_eq!(parts_false[0].value(), Some("false"));
}

#[test]
fn test_scalar_remaining_numeric_types() {
    // u8, u16, u128, usize
    assert_eq!(42u8.extract()[0].key(), "u8");
    assert_eq!(42u8.extract()[0].value(), Some("42"));

    assert_eq!(1000u16.extract()[0].key(), "u16");
    assert_eq!(1000u16.extract()[0].value(), Some("1000"));

    assert_eq!(1u128.extract()[0].key(), "u128");
    assert_eq!(1u128.extract()[0].value(), Some("1"));

    assert_eq!(99usize.extract()[0].key(), "usize");
    assert_eq!(99usize.extract()[0].value(), Some("99"));

    // i8, i16, i128, isize
    assert_eq!((-1i8).extract()[0].key(), "i8");
    assert_eq!((-1i8).extract()[0].value(), Some("-1"));

    assert_eq!((-500i16).extract()[0].key(), "i16");
    assert_eq!((-500i16).extract()[0].value(), Some("-500"));

    assert_eq!(1i128.extract()[0].key(), "i128");
    assert_eq!(1i128.extract()[0].value(), Some("1"));

    assert_eq!(0isize.extract()[0].key(), "isize");
    assert_eq!(0isize.extract()[0].value(), Some("0"));
}

#[test]
fn test_key_extract_for_ref() {
    // Exercise KeyExtract for &T (explicit trait call, not auto-deref)
    let value = 42u64;
    let ref_value: &u64 = &value;
    let parts = KeyExtract::extract(&ref_value);

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "u64");
    assert_eq!(parts[0].value(), Some("42"));
}

#[test]
fn test_scalar_option_some() {
    let value: Option<u64> = Some(42);
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "u64");
    assert_eq!(parts[0].value(), Some("42"));
}

#[test]
fn test_scalar_option_none() {
    let value: Option<u64> = None;
    let parts = value.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "none");
    assert_eq!(parts[0].value(), None);
}

#[tokio::test]
async fn test_extractor_with_scalars() {
    let extractor = FnExtractor::new("test::compute");
    let args = Args((42u64, "key".to_string()));

    let key_parts = extractor.get(args, &hitbox::EvalContext::new()).await;
    let (_, key) = key_parts.into_cache_key();

    let key_str = key.to_string();
    assert!(key_str.contains("test::compute"));
    assert!(key_str.contains("42"));
    assert!(key_str.contains("key"));
}

#[tokio::test]
async fn test_extractor_scalar_different_values() {
    let extractor = FnExtractor::new("test::add");

    let (_, key1) = extractor
        .get(Args((1i64, 2i64)), &hitbox::EvalContext::new())
        .await
        .into_cache_key();
    let (_, key2) = extractor
        .get(Args((1i64, 3i64)), &hitbox::EvalContext::new())
        .await
        .into_cache_key();
    let (_, key3) = extractor
        .get(Args((1i64, 2i64)), &hitbox::EvalContext::new())
        .await
        .into_cache_key();

    // Different args = different keys
    assert_ne!(key1.to_string(), key2.to_string());
    // Same args = same keys
    assert_eq!(key1.to_string(), key3.to_string());
}

// =============================================================================
// Reference type tests
// =============================================================================

#[test]
fn test_reference_to_derived_struct() {
    let user = UserId(42);
    let user_ref = &user;

    let parts = user_ref.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "0");
    assert_eq!(parts[0].value(), Some("42"));
}

#[test]
fn test_reference_to_scalar() {
    let value: u64 = 123;
    let value_ref = &value;

    let parts = value_ref.extract();

    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].key(), "u64");
    assert_eq!(parts[0].value(), Some("123"));
}

// =============================================================================
// Nested struct tests (recursive KeyExtract)
// =============================================================================

/// Nested struct containing another KeyExtract type.
/// Single-part inner field → key replaced with field name.
#[derive(KeyExtract)]
struct Request {
    user_id: UserId,
    tenant: TenantId,
}

/// Deeply nested struct with another KeyExtract struct as a field.
/// Multi-part inner field → keys prefixed with field name.
#[derive(KeyExtract)]
struct PaginatedRequest {
    req: Request,
    page: u32,
}

#[test]
fn test_nested_single_part_fields() {
    let req = Request {
        user_id: UserId(42),
        tenant: TenantId("acme".into()),
    };
    let parts = req.extract();

    // UserId returns 1 part → key replaced with "user_id"
    // TenantId returns 1 part → key replaced with "tenant"
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].key(), "user_id");
    assert_eq!(parts[0].value(), Some("42"));
    assert_eq!(parts[1].key(), "tenant");
    assert_eq!(parts[1].value(), Some("acme"));
}

#[test]
fn test_nested_multi_part_field() {
    let req = PaginatedRequest {
        req: Request {
            user_id: UserId(7),
            tenant: TenantId("corp".into()),
        },
        page: 3,
    };
    let parts = req.extract();

    // Request returns 2 parts → each prefixed with "req."
    // u32 returns 1 part → key replaced with "page"
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0].key(), "req.user_id");
    assert_eq!(parts[0].value(), Some("7"));
    assert_eq!(parts[1].key(), "req.tenant");
    assert_eq!(parts[1].value(), Some("corp"));
    assert_eq!(parts[2].key(), "page");
    assert_eq!(parts[2].value(), Some("3"));
}

/// Three levels of nesting.
#[derive(KeyExtract)]
struct ApiCall {
    request: PaginatedRequest,
    #[key_extract(name = "method")]
    http_method: String,
}

#[test]
fn test_deeply_nested_structs() {
    let call = ApiCall {
        request: PaginatedRequest {
            req: Request {
                user_id: UserId(1),
                tenant: TenantId("t".into()),
            },
            page: 5,
        },
        http_method: "GET".into(),
    };
    let parts = call.extract();

    // PaginatedRequest returns 3 parts → each prefixed with "request."
    // String returns 1 part → key replaced with "method"
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0].key(), "request.req.user_id");
    assert_eq!(parts[0].value(), Some("1"));
    assert_eq!(parts[1].key(), "request.req.tenant");
    assert_eq!(parts[1].value(), Some("t"));
    assert_eq!(parts[2].key(), "request.page");
    assert_eq!(parts[2].value(), Some("5"));
    assert_eq!(parts[3].key(), "method");
    assert_eq!(parts[3].value(), Some("GET"));
}
