//! Upstream service abstraction.
//!
//! This module provides the [`Upstream`] trait for calling backend services
//! when cache misses occur.
//!
//! ## Overview
//!
//! The `Upstream` trait abstracts over any async service that can handle
//! requests and return responses. This allows the caching layer to be
//! agnostic to the actual service implementation.
//!
//! ## Consuming Call
//!
//! The `call` method consumes the upstream by value. This is intentional:
//! the caching FSM calls upstream exactly once per request flow, so consuming
//! is semantically correct and avoids complex lifetime decoupling.
//!
//! ## Framework Integration
//!
//! Protocol-specific crates provide implementations for common frameworks:
//!
//! - `hitbox-reqwest` - Reqwest HTTP client integration
//! - `hitbox-tower` - Tower service integration

use std::future::Future;

/// Trait for calling upstream services with cacheable requests.
///
/// This trait is framework-agnostic and can be implemented for any async service.
/// The `call` method takes `self` by value — upstream is consumed when called.
///
/// # Examples
///
/// ```rust,ignore
/// use hitbox_core::Upstream;
/// use std::pin::Pin;
/// use std::future::Future;
///
/// struct MockUpstream {
///     response: MyResponse,
/// }
///
/// impl Upstream<MyRequest> for MockUpstream {
///     type Response = MyResponse;
///     type Future = Pin<Box<dyn Future<Output = Self::Response> + Send>>;
///
///     fn call(self, _req: MyRequest) -> Self::Future {
///         Box::pin(std::future::ready(self.response))
///     }
/// }
/// ```
pub trait Upstream<Req> {
    /// The response type returned by the upstream service.
    type Response;

    /// The future that resolves to the response.
    type Future: Future<Output = Self::Response> + Send;

    /// Call the upstream service with the given request, consuming the upstream.
    fn call(self, req: Req) -> Self::Future;
}
