//! Cache configuration traits.
//!
//! - [`CacheConfig`] — single cache configuration (predicates, extractor, policy)
//! - [`CacheConfigs`] — ordered collection of configurations for multi-config routing

use std::sync::Arc;

use crate::Extractor;
use crate::policy::PolicyConfig;
use crate::predicate::Predicate;
use crate::tag::TagExtractor;

/// Trait for cache configuration.
///
/// Provides predicates for determining cacheability, extractors for generating
/// cache keys, tag extractors for invalidation, and policy for TTL/staleness behavior.
pub trait CacheConfig<Req, Res> {
    /// Predicate type for filtering requests.
    type RequestPredicate: Predicate<Subject = Req> + Send + Sync + 'static;
    /// Predicate type for filtering responses.
    type ResponsePredicate: Predicate<Subject = Res> + Send + Sync + 'static;
    /// Extractor type for generating cache keys.
    type Extractor: Extractor<Subject = Req> + Send + Sync + 'static;
    /// Tag extractor type for deriving request-side cache tags.
    type RequestTagExtractor: TagExtractor<Subject = Req> + Send + Sync + 'static;
    /// Tag extractor type for deriving response-side cache tags.
    type ResponseTagExtractor: TagExtractor<Subject = Res> + Send + Sync + 'static;

    /// Returns predicates that decide if a request should be cached.
    fn request_predicates(&self) -> Self::RequestPredicate;
    /// Returns predicates that decide if a response should be cached.
    fn response_predicates(&self) -> Self::ResponsePredicate;
    /// Returns extractors that generate cache keys from requests.
    fn extractors(&self) -> Self::Extractor;
    /// Returns tag extractors that derive cache tags from requests.
    fn request_tag_extractors(&self) -> Self::RequestTagExtractor;
    /// Returns tag extractors that derive cache tags from responses.
    fn response_tag_extractors(&self) -> Self::ResponseTagExtractor;
    /// Returns TTL and behavior policy for cached entries.
    fn policy(&self) -> Arc<PolicyConfig>;
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
    type RequestTagExtractor = T::RequestTagExtractor;
    type ResponseTagExtractor = T::ResponseTagExtractor;

    fn request_predicates(&self) -> Self::RequestPredicate {
        self.as_ref().request_predicates()
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        self.as_ref().response_predicates()
    }

    fn extractors(&self) -> Self::Extractor {
        self.as_ref().extractors()
    }

    fn request_tag_extractors(&self) -> Self::RequestTagExtractor {
        self.as_ref().request_tag_extractors()
    }

    fn response_tag_extractors(&self) -> Self::ResponseTagExtractor {
        self.as_ref().response_tag_extractors()
    }

    fn policy(&self) -> Arc<PolicyConfig> {
        self.as_ref().policy()
    }
}
