use chrono::{DateTime, Utc};

#[cfg(any(test, feature = "test-helpers"))]
use std::sync::RwLock;

use crate::response::{CacheState, CacheableResponse};

#[cfg(any(test, feature = "test-helpers"))]
use crate::time_provider::TimeProvider;

// Global mock time provider for testing
// Available in test builds or when test-helpers feature is enabled
#[cfg(any(test, feature = "test-helpers"))]
static MOCK_TIME_PROVIDER: RwLock<Option<Box<dyn TimeProvider>>> = RwLock::new(None);

/// Set a mock time provider for testing.
///
/// This function is available in test builds or when the `test-helpers` feature
/// is enabled. It allows setting a custom time provider that will be used by
/// `CacheValue::cache_state()` instead of the real system time.
///
/// # Examples
///
/// ```ignore
/// use hitbox_core::set_mock_time_provider;
/// use hitbox_test::time::MockTimeProvider;
///
/// let mock_time = MockTimeProvider::new();
/// set_mock_time_provider(Some(Box::new(mock_time)));
///
/// // Now all cache_state calls will use the mock time
///
/// // Clear when done
/// set_mock_time_provider(None);
/// ```
#[cfg(any(test, feature = "test-helpers"))]
pub fn set_mock_time_provider(provider: Option<Box<dyn TimeProvider>>) {
    let mut mock = MOCK_TIME_PROVIDER.write().unwrap();
    *mock = provider;
}

/// Get the current time, using mock time provider if set (test/test-helpers only).
#[cfg(any(test, feature = "test-helpers"))]
fn current_time() -> DateTime<Utc> {
    let mock = MOCK_TIME_PROVIDER.read().unwrap();
    if let Some(provider) = mock.as_ref() {
        provider.now()
    } else {
        Utc::now()
    }
}

/// Get the current time (production version).
#[cfg(not(any(test, feature = "test-helpers")))]
#[inline]
fn current_time() -> DateTime<Utc> {
    Utc::now()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheValue<T> {
    pub data: T,
    pub stale: Option<DateTime<Utc>>,
    pub expire: Option<DateTime<Utc>>,
}

impl<T> CacheValue<T> {
    pub fn new(data: T, expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> Self {
        CacheValue {
            data,
            expire,
            stale,
        }
    }

    pub fn into_inner(self) -> T {
        self.data
    }

    pub fn into_parts(self) -> (CacheMeta, T) {
        (CacheMeta::new(self.expire, self.stale), self.data)
    }
}

impl<T> CacheValue<T> {
    pub async fn cache_state<C: CacheableResponse<Cached = T>>(self) -> CacheState<C> {
        let (meta, data) = self.into_parts();
        let origin = C::from_cached(data).await;
        let now = current_time();
        if let Some(expire) = meta.expire
            && expire <= now
        {
            CacheState::Expired(origin)
        } else if let Some(stale) = meta.stale
            && stale <= now
        {
            CacheState::Stale(origin)
        } else {
            CacheState::Actual(origin)
        }
    }
}

pub struct CacheMeta {
    pub expire: Option<DateTime<Utc>>,
    pub stale: Option<DateTime<Utc>>,
}

impl CacheMeta {
    pub fn new(expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> CacheMeta {
        CacheMeta { expire, stale }
    }
}
