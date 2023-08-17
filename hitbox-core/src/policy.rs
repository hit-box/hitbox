#[derive(Debug)]
pub enum CachePolicy<C, N> {
    Cacheable(C),
    NonCacheable(N),
}
