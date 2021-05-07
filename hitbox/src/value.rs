use crate::{CacheableResponse, CachePolicy};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize, Deserialize};

/// This struct wraps and represent cached data.
///
/// The expired field defines the UTC data expiration time.
#[derive(Deserialize)]
pub struct CachedValue<T> {
    data: T,
    expired: DateTime<Utc>,
}

#[derive(Serialize)]
struct CacheInnerValue<U> 
where
    U: Serialize
{
    data: U,
    expired: DateTime<Utc>,
}

impl<T> CachedValue<T> 
where
    T: CacheableResponse
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

    pub fn serialize(&self) -> Vec<u8> {
        match self.data.cache_policy() {
            CachePolicy::Cacheable(cache_value) => serde_json::to_vec(cache_value).unwrap(),
            CachePolicy::NonCacheable(_) => unreachable!(),
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
    /// Cached data is exisis and stale.
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
    fn from(cached_data: Option<CachedValue<U>>) -> Self {
        match cached_data {
            Some(cached_data) => Self::Actual(CachedValue::from_inner(cached_data)),
            None => Self::Miss,
        }
    }
}
