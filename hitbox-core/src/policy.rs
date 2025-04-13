use std::time::Duration;

#[derive(Debug)]
pub enum CachePolicy<C, N> {
    Cacheable(C),
    NonCacheable(N),
}

#[derive(Default)]
pub struct EntityPolicyConfig {
    pub ttl: Option<Duration>,
    pub stale_ttl: Option<Duration>,
}
