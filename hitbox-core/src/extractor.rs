//! Cache key extraction from requests.
//!
//! This module provides the [`Extractor`] trait for extracting data from
//! requests to build cache keys.
//!
//! ## Overview
//!
//! Extractors pull relevant data from requests (like HTTP method, path,
//! query parameters) and produce [`KeyParts`] that form the cache key.
//! Multiple extractors can be chained to build complex cache keys.
//!
//! ## Composability
//!
//! Extractors are designed to be composed. Protocol-specific crates like
//! `hitbox-http` provide extractors for common request components that
//! can be combined to create precise cache keys.
//!
//! ## Example
//!
//! ```ignore
//! use hitbox_core::{Extractor, KeyParts, KeyPart};
//!
//! #[derive(Debug)]
//! struct MethodExtractor;
//!
//! #[async_trait::async_trait]
//! impl Extractor for MethodExtractor {
//!     type Subject = HttpRequest;
//!     type Context = ();
//!
//!     async fn get(&self, request: Self::Subject, _ctx: &mut Self::Context) -> KeyParts<Self::Subject> {
//!         let mut parts = KeyParts::new(request);
//!         parts.push(KeyPart::new("method", Some(request.method().as_str())));
//!         parts
//!     }
//! }
//! ```

use std::sync::Arc;

use async_trait::async_trait;

use crate::KeyParts;

/// Trait for extracting cache key components from a subject.
///
/// Extractors are the mechanism for building cache keys from requests.
/// They are **protocol-agnostic** - the same trait works for HTTP requests,
/// gRPC messages, or any other protocol type.
///
/// # Type Parameters
///
/// The `Subject` associated type defines what this extractor processes.
/// Typically this is a request type from which cache key components
/// are extracted.
///
/// The `Context` associated type provides shared, pre-computed data.
/// See [`Predicate::Context`](crate::predicate::Predicate::Context) for details.
///
/// # Ownership
///
/// The `get` method takes ownership of the subject and returns it wrapped
/// in [`KeyParts`] along with the extracted key components. This allows
/// extractors to be chained without cloning.
///
/// # Blanket Implementations
///
/// This trait is implemented for:
/// - `&T` where `T: Extractor`
/// - `Box<T>` where `T: Extractor`
/// - `Arc<T>` where `T: Extractor`
#[async_trait]
pub trait Extractor {
    /// The type from which cache key components are extracted.
    type Subject;

    /// Shared context available to all extractors in a chain.
    ///
    /// Use `()` when no context is needed (zero-cost for HTTP).
    type Context: Default + Send;

    /// Extract cache key components from the subject.
    ///
    /// Returns a [`KeyParts`] containing the subject and accumulated key parts.
    /// The `ctx` parameter provides mutable access to a shared context.
    async fn get(&self, subject: Self::Subject, ctx: &mut Self::Context)
    -> KeyParts<Self::Subject>;
}

#[async_trait]
impl<T> Extractor for &T
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
    T::Context: Send,
{
    type Subject = T::Subject;
    type Context = T::Context;

    async fn get(&self, subject: T::Subject, ctx: &mut T::Context) -> KeyParts<T::Subject> {
        (**self).get(subject, ctx).await
    }
}

#[async_trait]
impl<T> Extractor for Box<T>
where
    T: Extractor + ?Sized + Sync,
    T::Subject: Send,
    T::Context: Send,
{
    type Subject = T::Subject;
    type Context = T::Context;

    async fn get(&self, subject: T::Subject, ctx: &mut T::Context) -> KeyParts<T::Subject> {
        self.as_ref().get(subject, ctx).await
    }
}

#[async_trait]
impl<T> Extractor for Arc<T>
where
    T: Extractor + Send + Sync + ?Sized,
    T::Subject: Send,
    T::Context: Send,
{
    type Subject = T::Subject;
    type Context = T::Context;

    async fn get(&self, subject: T::Subject, ctx: &mut T::Context) -> KeyParts<T::Subject> {
        self.as_ref().get(subject, ctx).await
    }
}
