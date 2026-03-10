//! Generator for #[cached] macro output.
//!
//! Each code generation part is encapsulated in its own struct implementing `ToTokens`.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::parser::CachedFn;

// =============================================================================
// Internal implementation function
// =============================================================================

/// Generates the internal async implementation function.
///
/// ```ignore
/// async fn __function_impl<'a, T: Clone>(args: Args<(Arg<&'a T0>, Arg<T>)>) -> ReturnType {
///     let Args((__arg0, __arg1)) = args;
///     let param0 = __arg0.into_value();
///     let param1 = __arg1.into_value();
///     // original body
/// }
/// ```
pub struct ImplFn<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> ImplFn<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for ImplFn<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let impl_name = &self.cached_fn.impl_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let args_pattern = self.cached_fn.args_destructure_pattern();
        let args_extract = self.cached_fn.args_extract_values();
        let return_type = &self.cached_fn.return_type;
        let body = &self.cached_fn.body;

        let generics = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.user_generic_params();
            quote! { <#generic_params> }
        } else {
            quote! {}
        };

        tokens.extend(quote! {
            async fn #impl_name #generics (args: hitbox_fn::Args<#args_tuple>) -> #return_type {
                let hitbox_fn::Args(#args_pattern) = args;
                #args_extract
                #body
            }
        });
    }
}

// =============================================================================
// Call struct definition
// =============================================================================

/// Generates the Call struct with type-state pattern.
///
/// ```ignore
/// pub struct FunctionCall<'a, T: Clone, B = NoBackend, P = NoPolicy, C = NoContext> {
///     args: Args<(Arg<&'a T0>, Arg<T>)>,
///     backend: B,
///     policy: P,
///     _context: PhantomData<C>,
/// }
/// ```
pub struct CallStruct<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallStruct<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallStruct<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let phantom_context = self.cached_fn.phantom_context_type();

        let generics = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            quote! { <#generic_params, B = hitbox_fn::NoBackend, P = hitbox_fn::NoPolicy, C = hitbox_fn::NoContext> }
        } else {
            quote! { <B = hitbox_fn::NoBackend, P = hitbox_fn::NoPolicy, C = hitbox_fn::NoContext> }
        };

        let vis = &self.cached_fn.vis;
        tokens.extend(quote! {
            #vis struct #call_name #generics {
                args: hitbox_fn::Args<#args_tuple>,
                backend: B,
                policy: P,
                _context: #phantom_context,
            }
        });
    }
}

// =============================================================================
// Call::new() implementation
// =============================================================================

/// Generates the `new()` constructor for Call struct.
pub struct CallImplNew<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallImplNew<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallImplNew<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_type();

        let (impl_generics, type_generics) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params> },
                quote! { <#generic_args, hitbox_fn::NoBackend, hitbox_fn::NoPolicy, hitbox_fn::NoContext> },
            )
        } else {
            (
                quote! {},
                quote! { <hitbox_fn::NoBackend, hitbox_fn::NoPolicy, hitbox_fn::NoContext> },
            )
        };

        tokens.extend(quote! {
            impl #impl_generics #call_name #type_generics {
                fn new(args: hitbox_fn::Args<#args_tuple>) -> Self {
                    Self {
                        args,
                        backend: hitbox_fn::NoBackend,
                        policy: hitbox_fn::NoPolicy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// Call::backend() implementation
// =============================================================================

/// Generates the `backend()` builder method.
pub struct CallImplBackend<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallImplBackend<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallImplBackend<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;

        let (impl_generics, type_generics_no_backend, type_generics_with_backend) =
            if self.cached_fn.has_generics() {
                let generic_params = self.cached_fn.generic_params();
                let generic_args = self.cached_fn.generic_args();
                (
                    quote! { <#generic_params, P, C> },
                    quote! { <#generic_args, hitbox_fn::NoBackend, P, C> },
                    quote! { <#generic_args, hitbox_fn::WithBackend<B>, P, C> },
                )
            } else {
                (
                    quote! { <P, C> },
                    quote! { <hitbox_fn::NoBackend, P, C> },
                    quote! { <hitbox_fn::WithBackend<B>, P, C> },
                )
            };

        tokens.extend(quote! {
            impl #impl_generics #call_name #type_generics_no_backend {
                pub fn backend<B: hitbox::backend::CacheBackend>(self, backend: B) -> #call_name #type_generics_with_backend {
                    #call_name {
                        args: self.args,
                        backend: hitbox_fn::WithBackend(std::sync::Arc::new(backend)),
                        policy: self.policy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// Call::policy() implementation
// =============================================================================

/// Generates the `policy()` builder method.
pub struct CallImplPolicy<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallImplPolicy<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallImplPolicy<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;

        let (impl_generics, type_generics_no_policy, type_generics_with_policy) =
            if self.cached_fn.has_generics() {
                let generic_params = self.cached_fn.generic_params();
                let generic_args = self.cached_fn.generic_args();
                (
                    quote! { <#generic_params, B, C> },
                    quote! { <#generic_args, B, hitbox_fn::NoPolicy, C> },
                    quote! { <#generic_args, B, hitbox_fn::WithPolicy, C> },
                )
            } else {
                (
                    quote! { <B, C> },
                    quote! { <B, hitbox_fn::NoPolicy, C> },
                    quote! { <B, hitbox_fn::WithPolicy, C> },
                )
            };

        tokens.extend(quote! {
            impl #impl_generics #call_name #type_generics_no_policy {
                pub fn policy(self, policy: hitbox::policy::PolicyConfig) -> #call_name #type_generics_with_policy {
                    #call_name {
                        args: self.args,
                        backend: self.backend,
                        policy: hitbox_fn::WithPolicy(std::sync::Arc::new(policy)),
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// Call::with_context() implementation
// =============================================================================

/// Generates the `with_context()` method for Call.
pub struct CallImplWithContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallImplWithContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallImplWithContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;

        let (impl_generics, type_generics_no_context, type_generics_with_context) =
            if self.cached_fn.has_generics() {
                let generic_params = self.cached_fn.generic_params();
                let generic_args = self.cached_fn.generic_args();
                (
                    quote! { <#generic_params, B, P> },
                    quote! { <#generic_args, B, P, hitbox_fn::NoContext> },
                    quote! { <#generic_args, B, P, hitbox_fn::WithContext> },
                )
            } else {
                (
                    quote! { <B, P> },
                    quote! { <B, P, hitbox_fn::NoContext> },
                    quote! { <B, P, hitbox_fn::WithContext> },
                )
            };

        tokens.extend(quote! {
            impl #impl_generics #call_name #type_generics_no_context {
                pub fn with_context(self) -> #call_name #type_generics_with_context {
                    #call_name {
                        args: self.args,
                        backend: self.backend,
                        policy: self.policy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// CachedCall struct definition
// =============================================================================

/// Generates the CachedCall struct for use with pre-configured Cache.
///
/// Uses a generic `CC` parameter instead of concrete `&'c Cache<B, CM, O>`,
/// enabling different cache ownership patterns via `CacheAccess` trait.
///
/// ```ignore
/// pub struct FunctionCallCached<'a, T: Clone, CC, C = NoContext> {
///     args: Args<(Arg<&'a T0>, Arg<T>)>,
///     cache: CC,
///     _context: PhantomData<C>,
/// }
/// ```
pub struct CachedCallStruct<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CachedCallStruct<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CachedCallStruct<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let phantom_context = self.cached_fn.phantom_context_type();

        let generics = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            quote! { <#generic_params, CC, C = hitbox_fn::NoContext> }
        } else {
            quote! { <CC, C = hitbox_fn::NoContext> }
        };

        let vis = &self.cached_fn.vis;
        tokens.extend(quote! {
            #vis struct #cached_call_name #generics {
                args: hitbox_fn::Args<#args_tuple>,
                cache: CC,
                _context: #phantom_context,
            }
        });
    }
}

// =============================================================================
// Call::cache() implementation
// =============================================================================

/// Generates the `cache()` method that transitions to CachedCall.
pub struct CallImplCache<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallImplCache<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallImplCache<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let cached_call_name = &self.cached_fn.cached_call_name;

        let (impl_generics, call_type_args, cached_type_args) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params, B, P, C> },
                quote! { <#generic_args, B, P, C> },
                quote! { <#generic_args, CC, C> },
            )
        } else {
            (
                quote! { <B, P, C> },
                quote! { <B, P, C> },
                quote! { <CC, C> },
            )
        };

        tokens.extend(quote! {
            impl #impl_generics #call_name #call_type_args {
                pub fn cache<CC>(self, cache: CC) -> #cached_call_name #cached_type_args {
                    #cached_call_name {
                        args: self.args,
                        cache,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// CachedCall::with_context() implementation
// =============================================================================

/// Generates the `with_context()` method for CachedCall.
pub struct CachedCallImplWithContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CachedCallImplWithContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CachedCallImplWithContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_call_name = &self.cached_fn.cached_call_name;

        let (impl_generics, type_args_no_context, type_args_with_context) =
            if self.cached_fn.has_generics() {
                let generic_params = self.cached_fn.generic_params();
                let generic_args = self.cached_fn.generic_args();
                (
                    quote! { <#generic_params, CC> },
                    quote! { <#generic_args, CC, hitbox_fn::NoContext> },
                    quote! { <#generic_args, CC, hitbox_fn::WithContext> },
                )
            } else {
                (
                    quote! { <CC> },
                    quote! { <CC, hitbox_fn::NoContext> },
                    quote! { <CC, hitbox_fn::WithContext> },
                )
            };

        tokens.extend(quote! {
            impl #impl_generics #cached_call_name #type_args_no_context {
                pub fn with_context(self) -> #cached_call_name #type_args_with_context {
                    #cached_call_name {
                        args: self.args,
                        cache: self.cache,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture passthrough (no backend, no policy, no context)
// =============================================================================

/// Generates `IntoFuture` impl for Call without any configuration.
/// Calls the underlying function directly — no caching.
pub struct IntoFuturePassthrough<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> IntoFuturePassthrough<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for IntoFuturePassthrough<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let impl_name = &self.cached_fn.impl_name;
        let return_type = &self.cached_fn.return_type;

        let (impl_generics, type_generics) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params> },
                quote! { <#generic_args, hitbox_fn::NoBackend, hitbox_fn::NoPolicy, hitbox_fn::NoContext> },
            )
        } else {
            (
                quote! {},
                quote! { <hitbox_fn::NoBackend, hitbox_fn::NoPolicy, hitbox_fn::NoContext> },
            )
        };

        let offload_lifetime = self.cached_fn.offload_lifetime();

        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #call_name #type_generics {
                type Output = #return_type;
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + #offload_lifetime>>;

                fn into_future(self) -> Self::IntoFuture {
                    let args = self.args;
                    Box::pin(async move {
                        #impl_name(args).await
                    })
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for Call (no context)
// =============================================================================

/// Generates `IntoFuture` impl for Call with backend and policy, without context.
pub struct IntoFutureCallNoContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> IntoFutureCallNoContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for IntoFutureCallNoContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let call_future_name = syn::Ident::new(
            &format!("{}Future", self.cached_fn.call_name),
            self.cached_fn.call_name.span(),
        );
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        let (impl_generics, type_generics) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params, B> },
                quote! { <#generic_args, hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::NoContext> },
            )
        } else {
            (
                quote! { <B> },
                quote! { <hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::NoContext> },
            )
        };

        let generic_args = self.cached_fn.generic_args();
        let offload_lifetime = self.cached_fn.offload_lifetime();
        let upstream_instance = self.cached_fn.upstream_instance();

        // Use named CallFutureStruct instead of Box::pin(async move { ... }).
        // The async block approach triggers "not general enough" errors with GAT lifetimes.
        let into_future_type = if self.cached_fn.has_generics() {
            quote! { #call_future_name<#generic_args, B, hitbox::concurrency::NoopConcurrencyManager, hitbox::DisabledOffload> }
        } else {
            quote! { #call_future_name<B, hitbox::concurrency::NoopConcurrencyManager, hitbox::DisabledOffload> }
        };

        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #call_name #type_generics
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                #return_type: hitbox::CacheableResponse + Send + #offload_lifetime,
                <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
            {
                type Output = #return_type;
                type IntoFuture = #into_future_type;

                fn into_future(self) -> Self::IntoFuture {
                    let upstream = #upstream_instance;
                    let extractor = hitbox_fn::FnExtractor::<hitbox_fn::Args<#args_tuple>>::new(#fn_path);

                    let cache_future = hitbox::fsm::CacheFuture::new(
                        self.backend.0,
                        self.args,
                        upstream,
                        hitbox::predicate::Neutral::new(),
                        hitbox::predicate::Neutral::new(),
                        extractor,
                        self.policy.0,
                        hitbox::DisabledOffload,
                        hitbox::concurrency::NoopConcurrencyManager,
                    );

                    #call_future_name { inner: cache_future }
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for Call (with context)
// =============================================================================

/// Generates `IntoFuture` impl for Call with backend, policy, and context.
pub struct IntoFutureCallWithContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> IntoFutureCallWithContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for IntoFutureCallWithContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        let (impl_generics, type_generics) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params, B> },
                quote! { <#generic_args, hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::WithContext> },
            )
        } else {
            (
                quote! { <B> },
                quote! { <hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::WithContext> },
            )
        };

        let offload_lifetime = self.cached_fn.offload_lifetime();
        let upstream_instance = self.cached_fn.upstream_instance();

        // Directly construct and box CacheFuture.
        // Avoids async block which triggers "not general enough" error with GATs.
        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #call_name #type_generics
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                #return_type: hitbox::CacheableResponse + Send + #offload_lifetime,
                <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + #offload_lifetime>>;

                fn into_future(self) -> Self::IntoFuture {
                    let upstream = #upstream_instance;
                    let extractor = hitbox_fn::FnExtractor::<hitbox_fn::Args<#args_tuple>>::new(#fn_path);

                    let cache_future = hitbox::fsm::CacheFuture::new(
                        self.backend.0,
                        self.args,
                        upstream,
                        hitbox::predicate::Neutral::new(),
                        hitbox::predicate::Neutral::new(),
                        extractor,
                        self.policy.0,
                        hitbox::DisabledOffload,
                        hitbox::concurrency::NoopConcurrencyManager,
                    );

                    Box::pin(cache_future)
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for CachedCall (no context)
// =============================================================================

/// Generates `IntoFuture` impl for CachedCall without context.
/// Uses `CC: CacheAccess` to abstract over cache ownership.
pub struct IntoFutureCachedNoContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> IntoFutureCachedNoContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for IntoFutureCachedNoContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let call_future_name = syn::Ident::new(
            &format!("{}Future", self.cached_fn.call_name),
            self.cached_fn.call_name.span(),
        );
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        let generic_args = self.cached_fn.generic_args();
        let offload_lifetime = self.cached_fn.offload_lifetime();
        let upstream_instance = self.cached_fn.upstream_instance();

        let (impl_generics, type_args) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            (
                quote! { <#generic_params, CC> },
                quote! { <#generic_args, CC, hitbox_fn::NoContext> },
            )
        } else {
            (quote! { <CC> }, quote! { <CC, hitbox_fn::NoContext> })
        };

        let future_type_args = if self.cached_fn.has_generics() {
            quote! { #generic_args, CC::Backend, CC::ConcurrencyManager, CC::Offload }
        } else {
            quote! { CC::Backend, CC::ConcurrencyManager, CC::Offload }
        };

        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #cached_call_name #type_args
            where
                CC: hitbox_fn::CacheAccess,
                CC::Backend: hitbox::backend::CacheBackend + Send + Sync + 'static,
                #return_type: hitbox::CacheableResponse + Send + 'static,
                <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                CC::ConcurrencyManager: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                CC::Offload: hitbox::Offload<#offload_lifetime> + Clone + #offload_lifetime,
            {
                type Output = #return_type;
                type IntoFuture = #call_future_name<#future_type_args>;

                fn into_future(self) -> Self::IntoFuture {
                    let upstream = #upstream_instance;
                    let extractor = hitbox_fn::FnExtractor::<hitbox_fn::Args<#args_tuple>>::new(#fn_path);

                    let cache_future = hitbox::fsm::CacheFuture::new(
                        self.cache.backend(),
                        self.args,
                        upstream,
                        hitbox::predicate::Neutral::new(),
                        hitbox::predicate::Neutral::new(),
                        extractor,
                        self.cache.policy(),
                        self.cache.offload(),
                        self.cache.concurrency_manager(),
                    );

                    #call_future_name { inner: cache_future }
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for CachedCall (with context)
// =============================================================================

/// Generates `IntoFuture` impl for CachedCall with context.
/// Uses `CC: CacheAccess` to abstract over cache ownership.
pub struct IntoFutureCachedWithContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> IntoFutureCachedWithContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for IntoFutureCachedWithContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        let offload_lifetime = self.cached_fn.offload_lifetime();
        let upstream_instance = self.cached_fn.upstream_instance();

        let (impl_generics, type_args) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (
                quote! { <#generic_params, CC> },
                quote! { <#generic_args, CC, hitbox_fn::WithContext> },
            )
        } else {
            (quote! { <CC> }, quote! { <CC, hitbox_fn::WithContext> })
        };

        // Directly construct and box CacheFuture.
        // Avoids async block which triggers "not general enough" error with GATs.
        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #cached_call_name #type_args
            where
                CC: hitbox_fn::CacheAccess,
                CC::Backend: hitbox::backend::CacheBackend + Send + Sync + 'static,
                #return_type: hitbox::CacheableResponse + Send + 'static,
                <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                CC::ConcurrencyManager: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                CC::Offload: hitbox::Offload<#offload_lifetime> + Clone + #offload_lifetime,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + #offload_lifetime>>;

                fn into_future(self) -> Self::IntoFuture {
                    let upstream = #upstream_instance;
                    let extractor = hitbox_fn::FnExtractor::<hitbox_fn::Args<#args_tuple>>::new(#fn_path);

                    let cache_future = hitbox::fsm::CacheFuture::new(
                        self.cache.backend(),
                        self.args,
                        upstream,
                        hitbox::predicate::Neutral::new(),
                        hitbox::predicate::Neutral::new(),
                        extractor,
                        self.cache.policy(),
                        self.cache.offload(),
                        self.cache.concurrency_manager(),
                    );

                    Box::pin(cache_future)
                }
            }
        });
    }
}

// =============================================================================
// Execute function
// =============================================================================

/// Generates the internal execute function that runs the cache FSM.
pub struct ExecuteFn<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> ExecuteFn<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for ExecuteFn<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let execute_name = &self.cached_fn.execute_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        let generics = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            quote! { <#generic_params, B, CM, O> }
        } else {
            quote! { <B, CM, O> }
        };

        let offload_lifetime = self.cached_fn.offload_lifetime();
        let upstream_instance = self.cached_fn.upstream_instance();

        tokens.extend(quote! {
            async fn #execute_name #generics (
                backend: std::sync::Arc<B>,
                policy: std::sync::Arc<hitbox::policy::PolicyConfig>,
                concurrency_manager: CM,
                offload: O,
                args: hitbox_fn::Args<#args_tuple>,
            ) -> (#return_type, hitbox::context::CacheContext)
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                CM: hitbox::concurrency::ConcurrencyManager<#return_type> + 'static,
                O: hitbox::Offload<#offload_lifetime> + #offload_lifetime,
            {
                let upstream = #upstream_instance;
                let extractor = hitbox_fn::FnExtractor::<hitbox_fn::Args<#args_tuple>>::new(#fn_path);

                let cache_future = hitbox::fsm::CacheFuture::new(
                    backend,
                    args,
                    upstream,
                    hitbox::predicate::Neutral::new(),
                    hitbox::predicate::Neutral::new(),
                    extractor,
                    policy,
                    offload,
                    concurrency_manager,
                );

                cache_future.await
            }
        });
    }
}

// =============================================================================
// Public wrapper function
// =============================================================================

/// Generates the public wrapper function that returns the Call struct.
pub struct PublicFn<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> PublicFn<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for PublicFn<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let vis = &self.cached_fn.vis;
        let name = &self.cached_fn.name;
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_expr();

        let params: Vec<_> = self
            .cached_fn
            .args
            .iter()
            .map(|arg| {
                let name = &arg.name;
                let ty = &arg.ty;
                quote! { #name: #ty }
            })
            .collect();

        let (fn_generics, call_type_generics) = if self.cached_fn.has_generics() {
            let generic_params = self.cached_fn.generic_params();
            let generic_args = self.cached_fn.generic_args();
            (quote! { <#generic_params> }, quote! { <#generic_args> })
        } else {
            (quote! {}, quote! {})
        };

        tokens.extend(quote! {
            #vis fn #name #fn_generics (#(#params),*) -> #call_name #call_type_generics {
                #call_name::new(hitbox_fn::Args(#args_tuple))
            }
        });
    }
}

// =============================================================================
// Upstream struct for boxed user future
// =============================================================================

/// Generates a dedicated upstream struct per function.
///
/// This struct implements `Upstream` trait and boxes only the user's async function
/// future, making the `CacheFuture` type fully nameable.
///
/// ```ignore
/// struct __FnUpstream;
///
/// impl<'a> hitbox::Upstream<Args<...>> for __FnUpstream {
///     type Response = ReturnType;
///     type Future = Pin<Box<dyn Future<Output = Self::Response> + Send + 'a>>;
///
///     fn call(self, args: Args<...>) -> Self::Future {
///         Box::pin(__fn_impl(args))
///     }
/// }
/// ```
pub struct UpstreamStruct<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> UpstreamStruct<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for UpstreamStruct<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let upstream_name = &self.cached_fn.upstream_name;
        let impl_name = &self.cached_fn.impl_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;

        let type_params = &self.cached_fn.type_params;
        let type_idents: Vec<_> = type_params.iter().map(|tp| &tp.ident).collect();

        let future_lifetime = self.cached_fn.offload_lifetime();

        // Build the function call with turbofish only if type parameters are present.
        // Lifetimes are late-bound and cannot be specified with turbofish.
        let fn_call = if !self.cached_fn.type_params.is_empty() {
            quote! { #impl_name::<#( #type_idents ),*>(args) }
        } else {
            quote! { #impl_name(args) }
        };

        // Build struct and impl generics compositionally.
        //
        // The struct carries only what it must own:
        // - `'__hitbox` if synthetic (compiler needs it as a type parameter)
        // - Type params (for turbofish instantiation)
        // User lifetimes are NOT on the struct — they're constrained by the
        // trait type parameter `Upstream<Args<(Arg<&'a str>,)>>` on the impl.
        let mut struct_params: Vec<TokenStream> = Vec::new();
        let mut struct_args: Vec<TokenStream> = Vec::new();
        let mut phantom_parts: Vec<TokenStream> = Vec::new();

        if self.cached_fn.needs_synthetic_lifetime() {
            let synthetic = self.cached_fn.synthetic_lifetime();
            struct_params.push(quote! { #synthetic });
            struct_args.push(quote! { #synthetic });
            phantom_parts.push(quote! { &#synthetic () });
        }
        if !type_idents.is_empty() {
            struct_params.push(quote! { #( #type_params ),* });
            struct_args.push(quote! { #( #type_idents ),* });
            phantom_parts.push(quote! { fn() -> (#( #type_idents, )*) });
        }

        if !struct_params.is_empty() {
            let generic_params = self.cached_fn.generic_params();

            tokens.extend(quote! {
                #[derive(Clone, Copy)]
                struct #upstream_name<#( #struct_params ),*>(
                    std::marker::PhantomData<(#( #phantom_parts, )*)>
                );

                impl<#generic_params> hitbox::Upstream<hitbox_fn::Args<#args_tuple>> for #upstream_name<#( #struct_args ),*> {
                    type Response = #return_type;
                    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Response> + Send + #future_lifetime>>;

                    fn call(self, args: hitbox_fn::Args<#args_tuple>) -> Self::Future {
                        Box::pin(#fn_call)
                    }
                }
            });
        } else if self.cached_fn.has_generics() {
            // Lifetimes only (no synthetic, no types) — unit struct, lifetimes on impl
            let generic_params = self.cached_fn.generic_params();

            tokens.extend(quote! {
                #[derive(Clone, Copy)]
                struct #upstream_name;

                impl<#generic_params> hitbox::Upstream<hitbox_fn::Args<#args_tuple>> for #upstream_name {
                    type Response = #return_type;
                    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Response> + Send + #future_lifetime>>;

                    fn call(self, args: hitbox_fn::Args<#args_tuple>) -> Self::Future {
                        Box::pin(#fn_call)
                    }
                }
            });
        } else {
            // No generics at all
            tokens.extend(quote! {
                #[derive(Clone, Copy)]
                struct #upstream_name;

                impl hitbox::Upstream<hitbox_fn::Args<#args_tuple>> for #upstream_name {
                    type Response = #return_type;
                    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Response> + Send + #future_lifetime>>;

                    fn call(self, args: hitbox_fn::Args<#args_tuple>) -> Self::Future {
                        Box::pin(#fn_call)
                    }
                }
            });
        }
    }
}

// =============================================================================
// CacheFuture type alias
// =============================================================================

/// Generates a type alias for the complex CacheFuture type.
///
/// ```ignore
/// type __FnCacheFuture<'a, B> = hitbox::fsm::CacheFuture<
///     'a, B, Args<...>, ReturnType, __FnUpstream, ...
/// >;
/// ```
pub struct CacheFutureTypeAlias<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CacheFutureTypeAlias<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CacheFutureTypeAlias<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cache_future_name = &self.cached_fn.cache_future_name;
        let upstream_name = &self.cached_fn.upstream_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;

        let lifetime_idents: Vec<_> = self
            .cached_fn
            .lifetimes
            .iter()
            .map(|lt| &lt.lifetime)
            .collect();
        let type_idents: Vec<_> = self
            .cached_fn
            .type_params
            .iter()
            .map(|tp| &tp.ident)
            .collect();

        // The CacheFuture lifetime parameter
        let cache_future_lifetime = self.cached_fn.offload_lifetime();

        // The upstream type with generic arguments.
        // Must match what UpstreamStruct puts on the struct:
        // - '__hitbox (synthetic) goes on struct
        // - Type params go on struct
        // - User lifetimes do NOT go on struct (they're on the impl only)
        // The upstream type — must match what UpstreamStruct puts on the struct.
        let upstream_type = {
            let mut args: Vec<TokenStream> = Vec::new();
            if self.cached_fn.needs_synthetic_lifetime() {
                let synthetic = self.cached_fn.synthetic_lifetime();
                args.push(quote! { #synthetic });
            }
            if !type_idents.is_empty() {
                args.push(quote! { #( #type_idents ),* });
            }
            if args.is_empty() {
                quote! { #upstream_name }
            } else {
                quote! { #upstream_name<#( #args ),*> }
            }
        };

        // Type alias params: includes all lifetimes (synthetic + user) and type params.
        // Type aliases can't have bounds, so we just list them.
        let type_alias_params = {
            let mut params: Vec<TokenStream> = Vec::new();
            if self.cached_fn.needs_synthetic_lifetime() {
                let synthetic = self.cached_fn.synthetic_lifetime();
                params.push(quote! { #synthetic });
            }
            if !lifetime_idents.is_empty() {
                params.push(quote! { #( #lifetime_idents ),* });
            }
            if !type_idents.is_empty() {
                params.push(quote! { #( #type_idents ),* });
            }
            if params.is_empty() {
                quote! {}
            } else {
                quote! { #( #params, )* }
            }
        };

        // Make CM and O generic so both Call and CachedCall paths can reuse this
        tokens.extend(quote! {
            type #cache_future_name<#type_alias_params B, CM, O> = hitbox::fsm::CacheFuture<
                #cache_future_lifetime,
                B,
                hitbox_fn::Args<#args_tuple>,
                #return_type,
                #upstream_type,
                hitbox::predicate::Neutral<hitbox_fn::Args<#args_tuple>>,
                hitbox::predicate::Neutral<<#return_type as hitbox::CacheableResponse>::Subject>,
                hitbox_fn::FnExtractor<hitbox_fn::Args<#args_tuple>>,
                CM,
                O,
            >;
        });
    }
}

// =============================================================================
// CallFuture struct (concrete future type for both Call and CachedCall paths)
// =============================================================================

/// Generates the CallFuture struct that implements Future directly.
///
/// This is the concrete future type returned by IntoFuture, avoiding Box<dyn Future>.
/// The CacheFuture is created immediately in `into_future()` and stored here.
///
/// Used by both `.backend().policy().await` and `.cache(&cache).await` paths.
///
/// ```ignore
/// #[pin_project]
/// pub struct FnCallFuture<'a, B, CM, O> {
///     #[pin]
///     inner: __FnCacheFuture<'a, B, CM, O>,
/// }
/// ```
pub struct CallFutureStruct<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> CallFutureStruct<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for CallFutureStruct<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_future_name = syn::Ident::new(
            &format!("{}Future", self.cached_fn.call_name),
            self.cached_fn.call_name.span(),
        );
        let cache_future_name = &self.cached_fn.cache_future_name;
        let return_type = &self.cached_fn.return_type;
        let args_tuple = self.cached_fn.args_tuple_type();

        let generic_params = self.cached_fn.generic_params();
        let generic_args = self.cached_fn.generic_args();
        let offload_lifetime = self.cached_fn.offload_lifetime();

        if self.cached_fn.has_generics() {
            tokens.extend(quote! {
                #[pin_project::pin_project]
                pub struct #call_future_name<#generic_params, B, CM, O>
                where
                    B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                    #return_type: hitbox::CacheableResponse + Send + 'static,
                    <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                    hitbox_fn::Args<#args_tuple>: hitbox::CacheableRequest + #offload_lifetime,
                    CM: hitbox::concurrency::ConcurrencyManager<#return_type> + 'static,
                    O: hitbox::Offload<#offload_lifetime> + #offload_lifetime,
                {
                    #[pin]
                    inner: #cache_future_name<#generic_args, B, CM, O>,
                }
            });
        } else {
            tokens.extend(quote! {
                #[pin_project::pin_project]
                pub struct #call_future_name<B, CM, O>
                where
                    B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                    #return_type: hitbox::CacheableResponse + Send + 'static,
                    <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                    hitbox_fn::Args<#args_tuple>: hitbox::CacheableRequest + #offload_lifetime,
                    CM: hitbox::concurrency::ConcurrencyManager<#return_type> + 'static,
                    O: hitbox::Offload<#offload_lifetime> + #offload_lifetime,
                {
                    #[pin]
                    inner: #cache_future_name<B, CM, O>,
                }
            });
        }
    }
}

// =============================================================================
// Future impl for CallFuture (no context)
// =============================================================================

/// Generates the Future implementation for CallFuture without context.
/// Used by both `.backend().policy().await` and `.cache(&cache).await` paths.
pub struct FutureImplCallNoContext<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> FutureImplCallNoContext<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }
}

impl ToTokens for FutureImplCallNoContext<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let call_future_name = syn::Ident::new(
            &format!("{}Future", self.cached_fn.call_name),
            self.cached_fn.call_name.span(),
        );
        let return_type = &self.cached_fn.return_type;

        let generic_params = self.cached_fn.generic_params();
        let generic_args = self.cached_fn.generic_args();
        let offload_lifetime = self.cached_fn.offload_lifetime();

        if self.cached_fn.has_generics() {
            tokens.extend(quote! {
                impl<#generic_params, B, CM, O> std::future::Future for #call_future_name<#generic_args, B, CM, O>
                where
                    B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                    #return_type: hitbox::CacheableResponse + Send + 'static,
                    <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                    CM: hitbox::concurrency::ConcurrencyManager<#return_type> + 'static,
                    O: hitbox::Offload<#offload_lifetime> + #offload_lifetime,
                {
                    type Output = #return_type;

                    fn poll(
                        self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Self::Output> {
                        self.project().inner.poll(cx).map(|(result, _ctx)| result)
                    }
                }
            });
        } else {
            tokens.extend(quote! {
                impl<B, CM, O> std::future::Future for #call_future_name<B, CM, O>
                where
                    B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                    #return_type: hitbox::CacheableResponse + Send + 'static,
                    <#return_type as hitbox::CacheableResponse>::Cached: hitbox::Cacheable + Send,
                    CM: hitbox::concurrency::ConcurrencyManager<#return_type> + 'static,
                    O: hitbox::Offload<#offload_lifetime> + #offload_lifetime,
                {
                    type Output = #return_type;

                    fn poll(
                        self: std::pin::Pin<&mut Self>,
                        cx: &mut std::task::Context<'_>,
                    ) -> std::task::Poll<Self::Output> {
                        self.project().inner.poll(cx).map(|(result, _ctx)| result)
                    }
                }
            });
        }
    }
}

// =============================================================================
// Main Generator
// =============================================================================

/// Main generator that composes all code generation parts.
pub struct Generator<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> Generator<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }

    pub fn generate(&self) -> TokenStream {
        let impl_fn = ImplFn::new(self.cached_fn);
        let upstream_struct = UpstreamStruct::new(self.cached_fn);
        let cache_future_type = CacheFutureTypeAlias::new(self.cached_fn);
        let call_struct = CallStruct::new(self.cached_fn);
        let call_impl_new = CallImplNew::new(self.cached_fn);
        let call_impl_backend = CallImplBackend::new(self.cached_fn);
        let call_impl_policy = CallImplPolicy::new(self.cached_fn);
        let call_impl_with_context = CallImplWithContext::new(self.cached_fn);
        let call_future_struct = CallFutureStruct::new(self.cached_fn);
        let future_impl_call_no_context = FutureImplCallNoContext::new(self.cached_fn);
        let cached_call_struct = CachedCallStruct::new(self.cached_fn);
        let call_impl_cache = CallImplCache::new(self.cached_fn);
        let cached_call_impl_with_context = CachedCallImplWithContext::new(self.cached_fn);
        let into_future_passthrough = IntoFuturePassthrough::new(self.cached_fn);
        let into_future_call_no_context = IntoFutureCallNoContext::new(self.cached_fn);
        let into_future_call_with_context = IntoFutureCallWithContext::new(self.cached_fn);
        let into_future_cached_no_context = IntoFutureCachedNoContext::new(self.cached_fn);
        let into_future_cached_with_context = IntoFutureCachedWithContext::new(self.cached_fn);
        let execute_fn = ExecuteFn::new(self.cached_fn);
        let public_fn = PublicFn::new(self.cached_fn);

        quote! {
            #impl_fn
            #upstream_struct
            #cache_future_type
            #call_struct
            #call_impl_new
            #call_impl_backend
            #call_impl_policy
            #call_impl_with_context
            #call_future_struct
            #future_impl_call_no_context
            #cached_call_struct
            #call_impl_cache
            #cached_call_impl_with_context
            #into_future_passthrough
            #into_future_call_no_context
            #into_future_call_with_context
            #into_future_cached_no_context
            #into_future_cached_with_context
            #execute_fn
            #public_fn
        }
    }
}

impl ToTokens for Generator<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(self.generate());
    }
}
