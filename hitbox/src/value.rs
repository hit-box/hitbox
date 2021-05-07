use crate::response::CacheableResponse;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Deserialize)]
pub struct CachedValue<T> {
    data: T,
    expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
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
    pub fn into_inner(self) -> T {
        self.data
    }
}

pub enum CacheState<T> {
    Actual(CachedValue<T>),
    Stale(CachedValue<T>),
    Miss,
}

impl<T, U> CacheState<T>
where
    T: CacheableResponse<Cached = U>,
    U: DeserializeOwned,
{
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
