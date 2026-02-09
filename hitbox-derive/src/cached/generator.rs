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
/// async fn __function_impl<T: Clone>(args: Args<(Arg<T0>, Arg<T>)>) -> ReturnType {
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
            let generic_params = self.cached_fn.generic_params();
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
/// pub struct FunctionCall<T: Clone, B = NoBackend, P = NoPolicy, C = NoContext> {
///     args: Args<(Arg<T0>, Arg<T>)>,
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
                _context: std::marker::PhantomData<C>,
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
/// ```ignore
/// pub struct FunctionCallCached<T: Clone, CC, C = NoContext> {
///     args: Args<(Arg<T0>, Arg<T>)>,
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
        let type_params = &self.cached_fn.type_params;

        let vis = &self.cached_fn.vis;
        tokens.extend(quote! {
            #vis struct #cached_call_name <#( #type_params, )* CC, C = hitbox_fn::NoContext> {
                args: hitbox_fn::Args<#args_tuple>,
                cache: CC,
                _context: std::marker::PhantomData<C>,
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

        let type_params = &self.cached_fn.type_params;
        let type_idents: Vec<_> = type_params.iter().map(|tp| &tp.ident).collect();

        tokens.extend(quote! {
            impl <#( #type_params, )* B, P, C> #call_name <#( #type_idents, )* B, P, C> {
                pub fn cache<CC>(self, cache: CC) -> #cached_call_name <#( #type_idents, )* CC, C> {
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

        let type_params = &self.cached_fn.type_params;
        let type_idents: Vec<_> = type_params.iter().map(|tp| &tp.ident).collect();

        tokens.extend(quote! {
            impl <#( #type_params, )* CC> #cached_call_name <#( #type_idents, )* CC, hitbox_fn::NoContext> {
                pub fn with_context(self) -> #cached_call_name <#( #type_idents, )* CC, hitbox_fn::WithContext> {
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
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

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

        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #call_name #type_generics
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
            {
                type Output = #return_type;
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = self.backend.0;
                    let policy = self.policy.0;
                    let args = self.args;

                    Box::pin(async move {
                        let (result, _ctx) = #execute_name(
                            backend,
                            policy,
                            hitbox::concurrency::NoopConcurrencyManager,
                            hitbox_core::DisabledOffload,
                            args
                        ).await;
                        result
                    })
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
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

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

        tokens.extend(quote! {
            impl #impl_generics std::future::IntoFuture for #call_name #type_generics
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = self.backend.0;
                    let policy = self.policy.0;
                    let args = self.args;

                    Box::pin(async move {
                        #execute_name(
                            backend,
                            policy,
                            hitbox::concurrency::NoopConcurrencyManager,
                            hitbox_core::DisabledOffload,
                            args
                        ).await
                    })
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for CachedCall (no context)
// =============================================================================

/// Generates `IntoFuture` impl for CachedCall without context.
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
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        let type_params = &self.cached_fn.type_params;
        let type_idents: Vec<_> = type_params.iter().map(|tp| &tp.ident).collect();

        tokens.extend(quote! {
            impl <#( #type_params, )* CC> std::future::IntoFuture for #cached_call_name <#( #type_idents, )* CC, hitbox_fn::NoContext>
            where
                CC: hitbox_fn::CacheAccess,
                CC::Backend: hitbox::backend::CacheBackend + Send + Sync + 'static,
                CC::ConcurrencyManager: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                CC::Offload: hitbox_core::Offload<'static> + Clone + 'static,
            {
                type Output = #return_type;
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = self.cache.backend();
                    let policy = self.cache.policy();
                    let concurrency_manager = self.cache.concurrency_manager();
                    let offload = self.cache.offload();
                    let args = self.args;

                    Box::pin(async move {
                        let (result, _ctx) = #execute_name(backend, policy, concurrency_manager, offload, args).await;
                        result
                    })
                }
            }
        });
    }
}

// =============================================================================
// IntoFuture for CachedCall (with context)
// =============================================================================

/// Generates `IntoFuture` impl for CachedCall with context.
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
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        let type_params = &self.cached_fn.type_params;
        let type_idents: Vec<_> = type_params.iter().map(|tp| &tp.ident).collect();

        tokens.extend(quote! {
            impl <#( #type_params, )* CC> std::future::IntoFuture for #cached_call_name <#( #type_idents, )* CC, hitbox_fn::WithContext>
            where
                CC: hitbox_fn::CacheAccess,
                CC::Backend: hitbox::backend::CacheBackend + Send + Sync + 'static,
                CC::ConcurrencyManager: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                CC::Offload: hitbox_core::Offload<'static> + Clone + 'static,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'static>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = self.cache.backend();
                    let policy = self.cache.policy();
                    let concurrency_manager = self.cache.concurrency_manager();
                    let offload = self.cache.offload();
                    let args = self.args;

                    Box::pin(async move {
                        #execute_name(backend, policy, concurrency_manager, offload, args).await
                    })
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
        let impl_name = &self.cached_fn.impl_name;
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
                O: hitbox_core::Offload<'static> + 'static,
            {
                let upstream = hitbox_fn::FnUpstream::new(#impl_name);
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
        let call_struct = CallStruct::new(self.cached_fn);
        let call_impl_new = CallImplNew::new(self.cached_fn);
        let call_impl_backend = CallImplBackend::new(self.cached_fn);
        let call_impl_policy = CallImplPolicy::new(self.cached_fn);
        let call_impl_with_context = CallImplWithContext::new(self.cached_fn);
        let cached_call_struct = CachedCallStruct::new(self.cached_fn);
        let call_impl_cache = CallImplCache::new(self.cached_fn);
        let cached_call_impl_with_context = CachedCallImplWithContext::new(self.cached_fn);
        let into_future_call_no_context = IntoFutureCallNoContext::new(self.cached_fn);
        let into_future_call_with_context = IntoFutureCallWithContext::new(self.cached_fn);
        let into_future_cached_no_context = IntoFutureCachedNoContext::new(self.cached_fn);
        let into_future_cached_with_context = IntoFutureCachedWithContext::new(self.cached_fn);
        let execute_fn = ExecuteFn::new(self.cached_fn);
        let public_fn = PublicFn::new(self.cached_fn);

        quote! {
            #impl_fn
            #call_struct
            #call_impl_new
            #call_impl_backend
            #call_impl_policy
            #call_impl_with_context
            #cached_call_struct
            #call_impl_cache
            #cached_call_impl_with_context
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
