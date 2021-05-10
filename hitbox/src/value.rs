use crate::{CacheError, CachePolicy, CacheableResponse};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::runtime::EvictionPolicy;

/// This struct wraps and represent cached data.
///
/// The expired field defines the UTC data expiration time.
#[derive(Deserialize)]
pub struct CachedValue<T> {
    data: T,
    expired: DateTime<Utc>,
}

impl<T> From<(T, EvictionPolicy)> for CachedValue<T> {
    fn from(model: (T, EvictionPolicy)) -> Self {
        let (data, eviction_policy) = model;
        match eviction_policy {
            EvictionPolicy::Ttl(settings) => {
                let duration = chrono::Duration::seconds(settings.stale_ttl as i64);
                let expired = chrono::Utc::now() + duration;
                Self { data, expired }
            }
        }
    }
}

#[derive(Serialize)]
struct CachedInnerValue<'a, U>
where
    U: Serialize,
{
    data: &'a U,
    expired: DateTime<Utc>,
}

impl<T> CachedValue<T>
where
    T: CacheableResponse,
{
    /// Creates new CachedValue
    pub fn new(data: T, expired: DateTime<Utc>) -> Self {
        Self { data, expired }
    }

    fn from_inner<U>(cached_data: CachedValue<U>) -> Self
    where
        T: CacheableResponse<Cached = U>,
    {
        Self {
            data: T::from_cached(cached_data.data),
            expired: cached_data.expired,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, CacheError> {
        match self.data.cache_policy() {
            CachePolicy::Cacheable(cache_value) => serde_json::to_vec(&CachedInnerValue {
                data: cache_value,
                expired: self.expired,
            }).map_err(CacheError::from),
            CachePolicy::NonCacheable(_) => Err(CacheError::DeserializeError)
        }
    }

    /// Returns original data from CachedValue
    pub fn into_inner(self) -> T {
        self.data
    }
}

/// Represents cuurent state of cached data.
pub enum CacheState<T> {
    /// Cached data is exists and actual.
    Actual(CachedValue<T>),
    /// Cached data is exists and stale.
    Stale(CachedValue<T>),
    /// Cached data is not exists.
    Miss,
}

impl<T, U> CacheState<T>
where
    T: CacheableResponse<Cached = U>,
    U: DeserializeOwned,
{
    /// Deserialize optional vector of bytes and check the actuality.
    pub fn from_bytes(bytes: Option<&Vec<u8>>) -> Result<Self, crate::CacheError> {
        let cached_data = bytes
            .map(|bytes| serde_json::from_slice::<CachedValue<U>>(bytes))
            .transpose()?;
        Ok(Self::from(cached_data))
    }
}

impl<T, U> From<Option<CachedValue<U>>> for CacheState<T>
where
    T: CacheableResponse<Cached = U>,
{
    fn from(cached_value: Option<CachedValue<U>>) -> Self {
        match cached_value {
            Some(value) => {
                if value.expired < Utc::now() {
                    Self::Stale(CachedValue::from_inner(value))
                } else {
                    Self::Actual(CachedValue::from_inner(value))
                }
            }
            None => Self::Miss,
        }
    }
}
