//! Pre-configured cache and typestate markers.

use std::sync::Arc;

use hitbox::DisabledOffload;
use hitbox::backend::CacheBackend;
use hitbox::concurrency::NoopConcurrencyManager;
use hitbox::policy::PolicyConfig;

/// Marker: no backend configured.
pub struct NoBackend;

/// Marker: no policy configured.
pub struct NoPolicy;

/// Marker: no context requested in response.
pub struct NoContext;

/// Marker: context requested in response.
pub struct WithContext;

/// Marker: backend configured.
pub struct WithBackend<B>(pub Arc<B>);

/// Marker: policy configured.
pub struct WithPolicy(pub Arc<PolicyConfig>);

/// Pre-configured cache with backend, policy, concurrency manager, and offload manager.
///
/// Use [`Cache::builder()`] to create a new cache configuration.
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::Cache;
/// use hitbox::policy::PolicyConfig;
/// use hitbox_moka::MokaBackend;
/// use std::time::Duration;
///
/// let backend = MokaBackend::builder().max_entries(1000).build();
/// let policy = PolicyConfig::builder().ttl(Duration::from_secs(60)).build();
///
/// let cache = Cache::builder()
///     .backend(backend)
///     .policy(policy)
///     .build();
///
/// // Use with cached functions
/// let result = my_cached_function(arg1, arg2)
///     .cache(&cache)
///     .await;
///
/// // With offload manager for Stale-While-Revalidate
/// use hitbox::offload::OffloadManager;
///
/// let cache = Cache::builder()
///     .backend(backend)
///     .policy(policy)
///     .offload(OffloadManager::with_defaults())
///     .build();
/// ```
pub struct Cache<B, CM = NoopConcurrencyManager, O = DisabledOffload> {
    pub(crate) backend: Arc<B>,
    pub(crate) policy: Arc<PolicyConfig>,
    pub(crate) concurrency_manager: CM,
    pub(crate) offload: O,
}

impl<B, CM, O> Cache<B, CM, O> {
    /// Get a reference to the backend Arc.
    pub fn backend(&self) -> &Arc<B> {
        &self.backend
    }

    /// Get a reference to the policy Arc.
    pub fn policy(&self) -> &Arc<PolicyConfig> {
        &self.policy
    }

    /// Get a reference to the concurrency manager.
    pub fn concurrency_manager(&self) -> &CM {
        &self.concurrency_manager
    }

    /// Get a reference to the offload manager.
    pub fn offload(&self) -> &O {
        &self.offload
    }
}

/// Trait for accessing cache internals.
///
/// This enables the generated `#[cached]` code to work with different
/// cache ownership patterns: `&Cache`, `Arc<Cache>`, etc.
pub trait CacheAccess {
    /// The cache backend type.
    type Backend;
    /// The concurrency manager type.
    type ConcurrencyManager;
    /// The offload manager type.
    type Offload;

    /// Get a shared reference to the backend.
    fn backend(&self) -> Arc<Self::Backend>;
    /// Get a shared reference to the policy.
    fn policy(&self) -> Arc<PolicyConfig>;
    /// Get the concurrency manager.
    fn concurrency_manager(&self) -> Self::ConcurrencyManager;
    /// Get the offload manager.
    fn offload(&self) -> Self::Offload;
}

impl<B, CM: Clone, O: Clone> CacheAccess for Cache<B, CM, O> {
    type Backend = B;
    type ConcurrencyManager = CM;
    type Offload = O;

    fn backend(&self) -> Arc<B> {
        Arc::clone(&self.backend)
    }
    fn policy(&self) -> Arc<PolicyConfig> {
        Arc::clone(&self.policy)
    }
    fn concurrency_manager(&self) -> CM {
        self.concurrency_manager.clone()
    }
    fn offload(&self) -> O {
        self.offload.clone()
    }
}

impl<T: CacheAccess> CacheAccess for &T {
    type Backend = T::Backend;
    type ConcurrencyManager = T::ConcurrencyManager;
    type Offload = T::Offload;

    fn backend(&self) -> Arc<Self::Backend> {
        (**self).backend()
    }
    fn policy(&self) -> Arc<PolicyConfig> {
        (**self).policy()
    }
    fn concurrency_manager(&self) -> Self::ConcurrencyManager {
        (**self).concurrency_manager()
    }
    fn offload(&self) -> Self::Offload {
        (**self).offload()
    }
}

impl<T: CacheAccess> CacheAccess for Arc<T> {
    type Backend = T::Backend;
    type ConcurrencyManager = T::ConcurrencyManager;
    type Offload = T::Offload;

    fn backend(&self) -> Arc<Self::Backend> {
        (**self).backend()
    }
    fn policy(&self) -> Arc<PolicyConfig> {
        (**self).policy()
    }
    fn concurrency_manager(&self) -> Self::ConcurrencyManager {
        (**self).concurrency_manager()
    }
    fn offload(&self) -> Self::Offload {
        (**self).offload()
    }
}

impl<B, CM: Clone, O: Clone> Clone for Cache<B, CM, O> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            policy: Arc::clone(&self.policy),
            concurrency_manager: self.concurrency_manager.clone(),
            offload: self.offload.clone(),
        }
    }
}

impl Cache<(), NoopConcurrencyManager, DisabledOffload> {
    /// Create a new cache builder.
    pub fn builder() -> CacheBuilder<NoBackend, NoPolicy, NoopConcurrencyManager, DisabledOffload> {
        CacheBuilder {
            backend: NoBackend,
            policy: NoPolicy,
            concurrency_manager: NoopConcurrencyManager,
            offload: DisabledOffload,
        }
    }
}

/// Builder for [`Cache`] with typestate pattern.
///
/// The builder ensures at compile time that both backend and policy are configured
/// before the cache can be built.
pub struct CacheBuilder<B, P, CM, O> {
    pub(crate) backend: B,
    pub(crate) policy: P,
    pub(crate) concurrency_manager: CM,
    pub(crate) offload: O,
}

impl<P, CM, O> CacheBuilder<NoBackend, P, CM, O> {
    /// Set the cache backend.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = Cache::builder()
    ///     .backend(MokaBackend::builder().build());
    /// ```
    pub fn backend<B: CacheBackend>(self, backend: B) -> CacheBuilder<WithBackend<B>, P, CM, O> {
        CacheBuilder {
            backend: WithBackend(Arc::new(backend)),
            policy: self.policy,
            concurrency_manager: self.concurrency_manager,
            offload: self.offload,
        }
    }
}

impl<B, CM, O> CacheBuilder<B, NoPolicy, CM, O> {
    /// Set the cache policy configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let builder = Cache::builder()
    ///     .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build());
    /// ```
    pub fn policy(self, policy: PolicyConfig) -> CacheBuilder<B, WithPolicy, CM, O> {
        CacheBuilder {
            backend: self.backend,
            policy: WithPolicy(Arc::new(policy)),
            concurrency_manager: self.concurrency_manager,
            offload: self.offload,
        }
    }
}

impl<B, P, CM, O> CacheBuilder<B, P, CM, O> {
    /// Set the concurrency manager for dogpile prevention.
    ///
    /// By default, [`NoopConcurrencyManager`] is used which doesn't prevent dogpile.
    /// Use [`BroadcastConcurrencyManager`](hitbox::concurrency::BroadcastConcurrencyManager)
    /// for production workloads.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hitbox::concurrency::BroadcastConcurrencyManager;
    ///
    /// let cache = Cache::builder()
    ///     .backend(backend)
    ///     .policy(policy)
    ///     .concurrency_manager(BroadcastConcurrencyManager::new())
    ///     .build();
    /// ```
    pub fn concurrency_manager<CM2>(self, cm: CM2) -> CacheBuilder<B, P, CM2, O> {
        CacheBuilder {
            backend: self.backend,
            policy: self.policy,
            concurrency_manager: cm,
            offload: self.offload,
        }
    }

    /// Set the offload manager for background task execution.
    ///
    /// By default, [`DisabledOffload`] is used which disables background revalidation.
    /// Use [`OffloadManager`](hitbox::offload::OffloadManager) to enable
    /// Stale-While-Revalidate pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hitbox::offload::OffloadManager;
    ///
    /// let cache = Cache::builder()
    ///     .backend(backend)
    ///     .policy(policy)
    ///     .offload(OffloadManager::with_defaults())
    ///     .build();
    /// ```
    pub fn offload<O2>(self, offload: O2) -> CacheBuilder<B, P, CM, O2> {
        CacheBuilder {
            backend: self.backend,
            policy: self.policy,
            concurrency_manager: self.concurrency_manager,
            offload,
        }
    }
}

impl<B, CM, O> CacheBuilder<WithBackend<B>, WithPolicy, CM, O>
where
    B: CacheBackend,
{
    /// Build the cache configuration.
    ///
    /// This method is only available when both backend and policy are configured.
    pub fn build(self) -> Cache<B, CM, O> {
        Cache {
            backend: self.backend.0,
            policy: self.policy.0,
            concurrency_manager: self.concurrency_manager,
            offload: self.offload,
        }
    }
}
