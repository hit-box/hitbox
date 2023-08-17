use chrono::{DateTime, Utc};

use crate::response::{CacheState, CacheableResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedValue<T> {
    pub data: T,
    pub expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    pub fn new(data: T, expired: DateTime<Utc>) -> Self {
        CachedValue { data, expired }
    }

    pub fn into_inner(self) -> T {
        self.data
    }
}

impl<T> CachedValue<T> {
    pub async fn cache_state<C: CacheableResponse<Cached = T>>(self) -> CacheState<C> {
        let origin = C::from_cached(self.data).await;
        CacheState::Actual(origin)
    }
}
