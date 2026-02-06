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
//! ## Reference Parameter Support
//!
//! The trait uses a Generic Associated Type (GAT) for the future, allowing
//! the future's lifetime to depend on the request lifetime. This enables
//! support for requests containing references.
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
/// It uses a Generic Associated Type (GAT) for the future to support requests
/// containing references.
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
///     type Future<'a> = Pin<Box<dyn Future<Output = Self::Response> + Send + 'a>>
///     where
///         Self: 'a,
///         MyRequest: 'a;
///
///     fn call<'a>(&mut self, _req: MyRequest) -> Self::Future<'a>
///     where
///         Self: 'a,
///         MyRequest: 'a,
///     {
///         Box::pin(std::future::ready(self.response.clone()))
///     }
/// }
/// ```
pub trait Upstream<Req> {
    /// The response type returned by the upstream service
    type Response;

    /// The future that resolves to the response.
    ///
    /// This is a Generic Associated Type (GAT) that allows the future's lifetime
    /// to depend on the request lifetime, enabling support for requests containing
    /// references.
    type Future<'a>: Future<Output = Self::Response> + Send + 'a
    where
        Self: 'a,
        Req: 'a;

    /// Call the upstream service with the given request.
    fn call<'a>(&mut self, req: Req) -> Self::Future<'a>
    where
        Self: 'a,
        Req: 'a;
}
