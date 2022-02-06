use chrono::{DateTime, Utc};

#[cfg_attr(test, derive(Clone, PartialEq, Debug))]
pub struct CachedValue<T> {
    pub(crate) data: T,
    pub(crate) expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    pub fn new(data: T, expired: DateTime<Utc>) -> Self {
        CachedValue { data, expired }
    }
}
