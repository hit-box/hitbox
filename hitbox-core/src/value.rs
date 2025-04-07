use chrono::{DateTime, Utc};

use crate::response::{CacheState, CacheableResponse};

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
        let origin = C::from_cached(self.data).await;
        CacheState::Actual(origin)
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
