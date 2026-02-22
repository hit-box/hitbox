//! Cache configuration traits.
//!
//! - [`CacheConfig`] — single cache configuration (predicates, extractor, policy)
//! - [`CacheConfigs`] — ordered collection of configurations for multi-config routing

use std::sync::Arc;

use crate::Extractor;
use crate::policy::PolicyConfig;
use crate::predicate::Predicate;

/// Trait for cache configuration.
///
/// Provides predicates for determining cacheability, extractors for generating
/// cache keys, and policy for TTL/staleness behavior.
pub trait CacheConfig<Req, Res> {
    /// Predicate type for filtering requests.
    type RequestPredicate: Predicate<Subject = Req> + Send + Sync + 'static;
    /// Predicate type for filtering responses.
    type ResponsePredicate: Predicate<Subject = Res> + Send + Sync + 'static;
    /// Extractor type for generating cache keys.
    type Extractor: Extractor<Subject = Req> + Send + Sync + 'static;

    /// Returns predicates that decide if a request should be cached.
    fn request_predicates(&self) -> Self::RequestPredicate;
    /// Returns predicates that decide if a response should be cached.
    fn response_predicates(&self) -> Self::ResponsePredicate;
    /// Returns extractors that generate cache keys from requests.
    fn extractors(&self) -> Self::Extractor;
    /// Returns TTL and behavior policy for cached entries.
    fn policy(&self) -> &PolicyConfig;
}

/// Trait for providing one or more cache configurations.
///
/// Configurations are evaluated in order; first match wins.
/// Use [`CacheConfig`] for a single configuration that wraps itself,
/// or a multi-config container like `SelectiveConfig` for routing.
pub trait CacheConfigs<Req, Res> {
    /// The individual config type.
    type Config: CacheConfig<Req, Res>;

    /// Returns the ordered list of configurations.
    fn configs(&self) -> &[Self::Config];
}

impl<T, Req, Res> CacheConfig<Req, Res> for Arc<T>
where
    T: CacheConfig<Req, Res>,
{
    type RequestPredicate = T::RequestPredicate;
    type ResponsePredicate = T::ResponsePredicate;
    type Extractor = T::Extractor;

    fn request_predicates(&self) -> Self::RequestPredicate {
        self.as_ref().request_predicates()
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        self.as_ref().response_predicates()
    }

    fn extractors(&self) -> Self::Extractor {
        self.as_ref().extractors()
    }

    fn policy(&self) -> &PolicyConfig {
        self.as_ref().policy()
    }
}
