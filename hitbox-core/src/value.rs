use chrono::{DateTime, Utc};

use crate::response::{CacheState, CacheableResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheValue<T> {
    pub data: T,
    pub expire: DateTime<Utc>,
}

impl<T> CacheValue<T> {
    pub fn new(data: T, expired: DateTime<Utc>) -> Self {
        CacheValue {
            data,
            expire: expired,
        }
    }

    pub fn into_inner(self) -> T {
        self.data
    }

    pub fn into_parts(self) -> (CacheMeta, T) {
        (CacheMeta::new(self.expire, self.expire), self.data)
    }
}

impl<T> CacheValue<T> {
    pub async fn cache_state<C: CacheableResponse<Cached = T>>(self) -> CacheState<C> {
        let origin = C::from_cached(self.data).await;
        CacheState::Actual(origin)
    }
}

pub struct CacheMeta {
    pub expire: DateTime<Utc>,
    stale: DateTime<Utc>,
}

impl CacheMeta {
    pub fn new(expire: DateTime<Utc>, stale: DateTime<Utc>) -> CacheMeta {
        CacheMeta { expire, stale }
    }
}
