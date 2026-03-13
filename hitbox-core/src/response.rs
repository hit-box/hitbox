//! Cacheable response types and traits.
//!
//! This module provides types for working with cacheable responses:
//!
//! - [`CacheableResponse`] - Trait for types that can be cached
//! - [`CacheState`] - Freshness state of cached data
//! - [`ResponseCachePolicy`] - Type alias for response cache decisions
//!
//! ## CacheableResponse Trait
//!
//! The [`CacheableResponse`] trait defines how response types are converted
//! to and from their cached representation. This allows responses to be
//! stored efficiently in cache backends.
//!
//! ## Cache States
//!
//! Cached data can be in three states:
//!
//! - [`CacheState::Actual`] - Data is fresh and valid
//! - [`CacheState::Stale`] - Data is usable but should be refreshed
//! - [`CacheState::Expired`] - Data is no longer valid
//!
//! ## Result Handling
//!
//! This module provides a blanket implementation of `CacheableResponse` for
//! `Result<T, E>` where `T: CacheableResponse`. This allows error responses
//! to pass through uncached while successful responses are cached.

use std::fmt::Debug;
use std::future::Future;

const POLL_AFTER_READY_ERROR: &str = "ResultIntoCachedFuture can't be polled after finishing";
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use chrono::Utc;
use pin_project::pin_project;

use crate::{
    CachePolicy, EntityPolicyConfig, EvalContext,
    predicate::{Predicate, PredicateResult},
    value::CacheValue,
};

/// Cache policy for responses.
///
/// Type alias that specializes [`CachePolicy`] for response caching:
/// - `Cacheable` variant contains a [`CacheValue`] with the cached representation
/// - `NonCacheable` variant contains the original response
pub type ResponseCachePolicy<C> = CachePolicy<CacheValue<<C as CacheableResponse>::Cached>, C>;

/// Freshness state of cached data.
///
/// Represents the time-based state of a cached value relative to its
/// staleness and expiration timestamps.
#[derive(Debug, PartialEq, Eq)]
pub enum CacheState<Cached> {
    /// Data is stale but not expired (usable, should refresh in background).
    Stale(Cached),
    /// Data is fresh and valid.
    Actual(Cached),
    /// Data has expired (must refresh before use).
    Expired(Cached),
}

/// Trait for response types that can be cached.
///
/// This trait defines how responses are converted to and from their cached
/// representation. Implementations must provide methods for:
///
/// - Determining if a response should be cached (`cache_policy`)
/// - Converting to the cached format (`into_cached`)
/// - Reconstructing from cached data (`from_cached`)
///
/// # Associated Types
///
/// - `Cached` - The serializable representation stored in cache
/// - `Subject` - The type that predicates evaluate (for wrapper types like `Result`)
/// - `IntoCachedFuture` - Future returned by `into_cached`
/// - `FromCachedFuture` - Future returned by `from_cached`
///
/// # Blanket Implementation
///
/// A blanket implementation is provided for `Result<T, E>` where `T: CacheableResponse`.
/// This allows:
/// - `Ok(response)` to be cached if the inner response is cacheable
/// - `Err(error)` to always pass through without caching
///
/// # Example Implementation
///
/// ```
/// use hitbox_core::{CacheableResponse, CachePolicy, EntityPolicyConfig, EvalContext};
/// use hitbox_core::predicate::{Predicate, PredicateResult};
/// use hitbox_core::response::ResponseCachePolicy;
/// use hitbox_core::value::CacheValue;
/// use chrono::Utc;
///
/// #[derive(Clone)]
/// struct MyResponse {
///     body: String,
///     status: u16,
/// }
///
/// impl CacheableResponse for MyResponse {
///     type Cached = String;
///     type Subject = Self;
///     type IntoCachedFuture = std::future::Ready<CachePolicy<String, Self>>;
///     type FromCachedFuture = std::future::Ready<Self>;
///
///     async fn cache_policy<P>(
///         self,
///         predicates: P,
///         config: &EntityPolicyConfig,
///     ) -> ResponseCachePolicy<Self>
///     where
///         P: Predicate<Subject = Self::Subject> + Send + Sync,
///     {
///         let ctx = EvalContext::new();
///         match predicates.check(self, &ctx).await {
///             PredicateResult::Cacheable(data) => {
///                 let cached = data.body.clone();
///                 CachePolicy::Cacheable(CacheValue::new(
///                     cached,
///                     config.ttl.map(|d| Utc::now() + d),
///                     config.stale_ttl.map(|d| Utc::now() + d),
///                 ))
///             }
///             PredicateResult::NonCacheable(data) => CachePolicy::NonCacheable(data),
///         }
///     }
///
///     fn into_cached(self) -> Self::IntoCachedFuture {
///         std::future::ready(CachePolicy::Cacheable(self.body))
///     }
///
///     fn from_cached(cached: String) -> Self::FromCachedFuture {
///         std::future::ready(MyResponse { body: cached, status: 200 })
///     }
/// }
/// ```
pub trait CacheableResponse
where
    Self: Sized + Send + 'static,
    Self::Cached: Clone,
{
    /// The serializable type stored in cache.
    type Cached;

    /// The type that response predicates evaluate.
    ///
    /// For simple responses, this is `Self`. For wrapper types like `Result<T, E>`,
    /// this is the inner type `T`.
    type Subject: CacheableResponse;

    /// Future type for `into_cached` method.
    type IntoCachedFuture: Future<Output = CachePolicy<Self::Cached, Self>> + Send;

    /// Future type for `from_cached` method.
    type FromCachedFuture: Future<Output = Self> + Send;

    /// Determine if this response should be cached.
    ///
    /// Applies predicates to determine cacheability, then converts cacheable
    /// responses to their cached representation with TTL metadata.
    ///
    /// # Arguments
    ///
    /// * `predicates` - Predicates to evaluate whether the response is cacheable
    /// * `config` - TTL configuration for the cached entry
    fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> impl Future<Output = ResponseCachePolicy<Self>> + Send
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync;

    /// Convert this response to its cached representation.
    ///
    /// Returns `Cacheable` with the serializable data, or `NonCacheable`
    /// if the response should not be cached.
    fn into_cached(self) -> Self::IntoCachedFuture;

    /// Reconstruct a response from cached data.
    ///
    /// Creates a new response instance from previously cached data.
    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture;
}

// =============================================================================
// Scalar type implementations
// =============================================================================

macro_rules! impl_cacheable_response_for_scalar {
    ($($ty:ty),* $(,)?) => {
        $(
            impl CacheableResponse for $ty {
                type Cached = Self;
                type Subject = Self;
                type IntoCachedFuture = std::future::Ready<CachePolicy<Self, Self>>;
                type FromCachedFuture = std::future::Ready<Self>;

                async fn cache_policy<P>(
                    self,
                    predicates: P,
                    config: &EntityPolicyConfig,
                ) -> ResponseCachePolicy<Self>
                where
                    P: Predicate<Subject = Self::Subject> + Send + Sync,
                {
                    let ctx = EvalContext::new();
                    match predicates.check(self, &ctx).await {
                        PredicateResult::Cacheable(data) => {
                            let cached = data.clone();
                            CachePolicy::Cacheable(CacheValue::new(
                                cached,
                                config.ttl.map(|d| Utc::now() + d),
                                config.stale_ttl.map(|d| Utc::now() + d),
                            ))
                        }
                        PredicateResult::NonCacheable(data) => CachePolicy::NonCacheable(data),
                    }
                }

                fn into_cached(self) -> Self::IntoCachedFuture {
                    std::future::ready(CachePolicy::Cacheable(self))
                }

                fn from_cached(cached: Self) -> Self::FromCachedFuture {
                    std::future::ready(cached)
                }
            }
        )*
    };
}

impl_cacheable_response_for_scalar! {
    // Unsigned integers
    u8, u16, u32, u64, u128, usize,
    // Signed integers
    i8, i16, i32, i64, i128, isize,
    // Other primitives
    bool, char,
    // Common types
    String,
}

// =============================================================================
// Vec<T> implementation
// =============================================================================

impl<T> CacheableResponse for Vec<T>
where
    T: CacheableResponse<Cached = T, Subject = T> + Clone + Send + 'static,
{
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<CachePolicy<Self, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
    {
        let ctx = EvalContext::new();
        match predicates.check(self, &ctx).await {
            PredicateResult::Cacheable(data) => {
                let cached = data.clone();
                CachePolicy::Cacheable(CacheValue::new(
                    cached,
                    config.ttl.map(|d| Utc::now() + d),
                    config.stale_ttl.map(|d| Utc::now() + d),
                ))
            }
            PredicateResult::NonCacheable(data) => CachePolicy::NonCacheable(data),
        }
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        std::future::ready(CachePolicy::Cacheable(self))
    }

    fn from_cached(cached: Self) -> Self::FromCachedFuture {
        std::future::ready(cached)
    }
}

// =============================================================================
// Result<T, E> wrapper futures
// =============================================================================

#[doc(hidden)]
#[pin_project(project = ResultIntoCachedProj)]
pub enum ResultIntoCachedFuture<T, E>
where
    T: CacheableResponse,
{
    /// Ok variant - wraps the inner type's future
    Ok(#[pin] T::IntoCachedFuture),
    /// Err variant - contains the error to return immediately
    Err(Option<E>),
}

impl<T, E> Future for ResultIntoCachedFuture<T, E>
where
    T: CacheableResponse,
{
    type Output = CachePolicy<T::Cached, Result<T, E>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            ResultIntoCachedProj::Ok(fut) => fut.poll(cx).map(|policy| match policy {
                CachePolicy::Cacheable(res) => CachePolicy::Cacheable(res),
                CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
            }),
            ResultIntoCachedProj::Err(e) => Poll::Ready(CachePolicy::NonCacheable(Err(e
                .take()
                .expect(POLL_AFTER_READY_ERROR)))),
        }
    }
}

#[doc(hidden)]
#[pin_project]
pub struct ResultFromCachedFuture<T, E>
where
    T: CacheableResponse,
{
    #[pin]
    inner: T::FromCachedFuture,
    _marker: PhantomData<E>,
}

impl<T, E> Future for ResultFromCachedFuture<T, E>
where
    T: CacheableResponse,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll(cx).map(Ok)
    }
}

// =============================================================================
// Result<T, E> implementation
// =============================================================================

impl<T, E> CacheableResponse for Result<T, E>
where
    T: CacheableResponse + 'static,
    E: Send + 'static,
    T::Cached: Send,
{
    type Cached = <T as CacheableResponse>::Cached;
    type Subject = T;
    type IntoCachedFuture = ResultIntoCachedFuture<T, E>;
    type FromCachedFuture = ResultFromCachedFuture<T, E>;

    async fn cache_policy<P>(
        self,
        predicates: P,
        config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
    {
        let ctx = EvalContext::new();
        match self {
            Ok(response) => match predicates.check(response, &ctx).await {
                PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                    CachePolicy::Cacheable(res) => CachePolicy::Cacheable(CacheValue::new(
                        res,
                        config.ttl.map(|duration| Utc::now() + duration),
                        config.stale_ttl.map(|duration| Utc::now() + duration),
                    )),
                    CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
                },
                PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(Ok(res)),
            },
            Err(error) => ResponseCachePolicy::NonCacheable(Err(error)),
        }
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        match self {
            Ok(response) => ResultIntoCachedFuture::Ok(response.into_cached()),
            Err(error) => ResultIntoCachedFuture::Err(Some(error)),
        }
    }

    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
        ResultFromCachedFuture {
            inner: T::from_cached(cached),
            _marker: PhantomData,
        }
    }
}
