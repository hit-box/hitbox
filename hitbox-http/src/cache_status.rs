//! Cache status extension for HTTP responses.
//!
//! This module provides the [`CacheStatusExt`] implementation for HTTP responses,
//! allowing cache status information to be attached as headers.
//!
//! Two headers are generated:
//!
//! - **`Cache-Status`** (RFC 9211) — structured field describing what the cache did
//! - **`Age`** (RFC 9111 §5.1) — how long the response has been in cache
//! - **`x-cache-status`** (legacy) — simple HIT/MISS/STALE string (kept for backward compatibility)

use std::fmt::Write;

use chrono::Utc;
use hitbox::{CacheContext, CacheStatus, CacheStatusExt, ForwardReason};
use http::header::HeaderName;
use http::{HeaderValue, header};
use hyper::body::Body as HttpBody;

use crate::CacheableHttpResponse;

/// Default header name for the legacy cache status header (HIT/MISS/STALE).
///
/// The value is `x-cache-status`. Use builder methods on cache middleware
/// to customize the header name.
pub const DEFAULT_CACHE_STATUS_HEADER: HeaderName = HeaderName::from_static("x-cache-status");

/// Default cache name used in the RFC 9211 `Cache-Status` header.
pub const DEFAULT_CACHE_NAME: &str = "hitbox";

/// Header name for the RFC 9211 Cache-Status structured header.
pub const CACHE_STATUS_HEADER: HeaderName = HeaderName::from_static("cache-status");

/// HTTP-specific cache data stored in [`CacheContext`] extensions.
///
/// Protocol-specific data that doesn't belong in the protocol-agnostic
/// `CacheContext` but is needed for HTTP header generation.
#[derive(Debug, Clone, Copy)]
pub struct HttpCacheData {
    /// Upstream HTTP response status code (for `fwd-status` parameter in RFC 9211).
    pub upstream_status: u16,
}

/// Configuration for HTTP cache status headers.
#[derive(Debug, Clone)]
pub struct HttpCacheStatusConfig {
    /// Header name for the legacy `x-cache-status` header.
    /// Set to `None` to disable the legacy header.
    pub legacy_header: Option<HeaderName>,
    /// Cache name used in the RFC 9211 `Cache-Status` header (default: "hitbox").
    pub cache_name: String,
}

impl Default for HttpCacheStatusConfig {
    fn default() -> Self {
        Self {
            legacy_header: Some(DEFAULT_CACHE_STATUS_HEADER),
            cache_name: DEFAULT_CACHE_NAME.to_string(),
        }
    }
}

impl HttpCacheStatusConfig {
    /// Creates a config with a custom cache name.
    pub fn with_cache_name(mut self, name: impl Into<String>) -> Self {
        self.cache_name = name.into();
        self
    }

    /// Creates a config with a custom legacy header name.
    pub fn with_legacy_header(mut self, header: HeaderName) -> Self {
        self.legacy_header = Some(header);
        self
    }

    /// Disables the legacy `x-cache-status` header.
    pub fn without_legacy_header(mut self) -> Self {
        self.legacy_header = None;
        self
    }
}

/// Formats the RFC 9211 `Cache-Status` header value.
///
/// See: <https://www.rfc-editor.org/rfc/rfc9211>
fn format_cache_status(ctx: &CacheContext, cache_name: &str) -> String {
    let mut buf = String::with_capacity(64);
    buf.push_str(cache_name);

    match ctx.status {
        CacheStatus::Hit | CacheStatus::Stale | CacheStatus::Collapsed => {
            buf.push_str("; hit");

            // Add ttl (can be negative for stale responses)
            if let Some(timing) = &ctx.timing
                && let Some(expire) = timing.expire
            {
                let ttl = (expire - Utc::now()).num_seconds();
                let _ = write!(buf, "; ttl={ttl}");
            }

            // Collapsed requests get the collapsed parameter
            if ctx.status == CacheStatus::Collapsed {
                buf.push_str("; collapsed");
            }
        }
        CacheStatus::Forward(reason) => {
            let fwd_value = match reason {
                // RFC 9211: fwd=stale means "had stale data but forwarded anyway"
                ForwardReason::Expired => "stale",
                ForwardReason::Miss => "miss",
                ForwardReason::Bypass => "bypass",
            };
            let _ = write!(buf, "; fwd={fwd_value}");

            // Add fwd-status from protocol extensions
            if let Some(ext) = &ctx.extensions
                && let Some(http_data) = ext.downcast_ref::<HttpCacheData>()
            {
                let _ = write!(buf, "; fwd-status={}", http_data.upstream_status);
            }
        }
    }

    // Stored flag
    if ctx.stored {
        buf.push_str("; stored");
    }

    buf
}

/// Computes the `Age` header value in seconds.
///
/// Returns `None` if the response wasn't served from cache.
/// Per RFC 9111 §5.1, caches MUST generate an Age header in responses served from cache.
fn compute_age(ctx: &CacheContext) -> Option<u64> {
    if !ctx.status.is_served_from_cache() {
        return None;
    }

    ctx.timing.map(|timing| {
        let age = (Utc::now() - timing.created_at).num_seconds().max(0);
        age as u64
    })
}

impl<ResBody> CacheStatusExt for CacheableHttpResponse<ResBody>
where
    ResBody: HttpBody,
{
    type Config = HttpCacheStatusConfig;

    fn cache_status(&mut self, context: &CacheContext, config: &Self::Config) {
        // RFC 9211: Cache-Status structured header
        let cache_status_value = format_cache_status(context, &config.cache_name);
        if let Ok(value) = HeaderValue::from_str(&cache_status_value) {
            self.parts
                .headers
                .insert(CACHE_STATUS_HEADER.clone(), value);
        }

        // RFC 9111 §5.1: Age header (only for responses served from cache)
        if let Some(age) = compute_age(context)
            && let Ok(value) = HeaderValue::from_str(&age.to_string())
        {
            self.parts.headers.insert(header::AGE, value);
        }

        // Legacy x-cache-status header (backward compatibility)
        if let Some(ref legacy_header) = config.legacy_header {
            let value = match context.status {
                CacheStatus::Hit => HeaderValue::from_static("HIT"),
                CacheStatus::Collapsed => HeaderValue::from_static("HIT"),
                CacheStatus::Stale => HeaderValue::from_static("STALE"),
                CacheStatus::Forward(_) => HeaderValue::from_static("MISS"),
            };
            self.parts.headers.insert(legacy_header.clone(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use hitbox::CacheTiming;

    #[test]
    fn test_format_cache_hit() {
        let created = Utc::now() - Duration::seconds(900);
        let expire = Utc::now() + Duration::seconds(2700);

        let ctx = CacheContext {
            status: CacheStatus::Hit,
            timing: Some(CacheTiming {
                created_at: created,
                expire: Some(expire),
            }),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert!(result.starts_with("hitbox; hit; ttl="));
        // ttl should be approximately 2700 (±1 for timing)
        let ttl_str = result.strip_prefix("hitbox; hit; ttl=").unwrap();
        let ttl: i64 = ttl_str.parse().unwrap();
        assert!((2699..=2701).contains(&ttl));
    }

    #[test]
    fn test_format_cache_miss() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Miss),
            stored: true,
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; fwd=miss; stored");
    }

    #[test]
    fn test_format_cache_miss_with_fwd_status() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Miss),
            stored: true,
            extensions: Some(Box::new(HttpCacheData {
                upstream_status: 200,
            })),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; fwd=miss; fwd-status=200; stored");
    }

    #[test]
    fn test_format_cache_bypass() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Bypass),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; fwd=bypass");
    }

    #[test]
    fn test_format_stale_swr() {
        let created = Utc::now() - Duration::seconds(3720);
        let expire = Utc::now() - Duration::seconds(120);

        let ctx = CacheContext {
            status: CacheStatus::Stale,
            timing: Some(CacheTiming {
                created_at: created,
                expire: Some(expire),
            }),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert!(result.starts_with("hitbox; hit; ttl=-"));
        let ttl_str = result.strip_prefix("hitbox; hit; ttl=").unwrap();
        let ttl: i64 = ttl_str.parse().unwrap();
        assert!(((-121)..=(-119)).contains(&ttl));
    }

    #[test]
    fn test_format_collapsed() {
        let ctx = CacheContext {
            status: CacheStatus::Collapsed,
            timing: Some(CacheTiming {
                created_at: Utc::now(),
                expire: None,
            }),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; hit; collapsed");
    }

    #[test]
    fn test_format_expired_forward() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Expired),
            stored: true,
            extensions: Some(Box::new(HttpCacheData {
                upstream_status: 200,
            })),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; fwd=stale; fwd-status=200; stored");
    }

    #[test]
    fn test_format_custom_cache_name() {
        let ctx = CacheContext {
            status: CacheStatus::Hit,
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "my-proxy");
        assert_eq!(result, "my-proxy; hit");
    }

    #[test]
    fn test_compute_age_cache_hit() {
        let ctx = CacheContext {
            status: CacheStatus::Hit,
            timing: Some(CacheTiming {
                created_at: Utc::now() - Duration::seconds(900),
                expire: Some(Utc::now() + Duration::seconds(2700)),
            }),
            ..Default::default()
        };

        let age = compute_age(&ctx).unwrap();
        assert!((899..=901).contains(&age));
    }

    #[test]
    fn test_compute_age_cache_miss_returns_none() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Miss),
            ..Default::default()
        };

        assert!(compute_age(&ctx).is_none());
    }

    #[test]
    fn test_compute_age_no_timing_returns_none() {
        let ctx = CacheContext {
            status: CacheStatus::Hit,
            timing: None,
            ..Default::default()
        };

        assert!(compute_age(&ctx).is_none());
    }

    #[test]
    fn test_format_upstream_error_not_stored() {
        let ctx = CacheContext {
            status: CacheStatus::Forward(ForwardReason::Miss),
            stored: false,
            extensions: Some(Box::new(HttpCacheData {
                upstream_status: 500,
            })),
            ..Default::default()
        };

        let result = format_cache_status(&ctx, "hitbox");
        assert_eq!(result, "hitbox; fwd=miss; fwd-status=500");
    }
}
