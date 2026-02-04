use proc_macro2::TokenStream;
use quote::quote;

use super::parser::CachedFn;

pub struct Generator<'a> {
    cached_fn: &'a CachedFn,
}

impl<'a> Generator<'a> {
    pub fn new(cached_fn: &'a CachedFn) -> Self {
        Self { cached_fn }
    }

    pub fn generate(&self) -> TokenStream {
        let impl_fn = self.generate_impl_fn();
        let call_struct = self.generate_call_struct();
        let call_impl_new = self.generate_call_impl_new();
        let call_impl_backend = self.generate_call_impl_backend();
        let call_impl_policy = self.generate_call_impl_policy();
        let call_impl_with_context = self.generate_call_impl_with_context();
        let cached_call_struct = self.generate_cached_call_struct();
        let call_impl_cache = self.generate_call_impl_cache();
        let cached_call_impl_with_context = self.generate_cached_call_impl_with_context();
        let into_future_call_no_context = self.generate_into_future_call_no_context();
        let into_future_call_with_context = self.generate_into_future_call_with_context();
        let into_future_cached_no_context = self.generate_into_future_cached_no_context();
        let into_future_cached_with_context = self.generate_into_future_cached_with_context();
        let execute_fn = self.generate_execute_fn();
        let public_fn = self.generate_public_fn();

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

    fn generate_impl_fn(&self) -> TokenStream {
        let impl_name = &self.cached_fn.impl_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let args_destructure = self.cached_fn.args_destructure();
        let return_type = &self.cached_fn.return_type;
        let body = &self.cached_fn.body;

        quote! {
            async fn #impl_name(args: hitbox_fn::Args<#args_tuple>) -> #return_type {
                let hitbox_fn::Args(#args_destructure) = args;
                #body
            }
        }
    }

    fn generate_call_struct(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_type();

        quote! {
            pub struct #call_name<B = hitbox_fn::NoBackend, P = hitbox_fn::NoPolicy, C = hitbox_fn::NoContext> {
                args: hitbox_fn::Args<#args_tuple>,
                backend: B,
                policy: P,
                _context: std::marker::PhantomData<C>,
            }
        }
    }

    fn generate_call_impl_new(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;
        let args_tuple = self.cached_fn.args_tuple_type();

        quote! {
            impl #call_name<hitbox_fn::NoBackend, hitbox_fn::NoPolicy, hitbox_fn::NoContext> {
                fn new(args: hitbox_fn::Args<#args_tuple>) -> Self {
                    Self {
                        args,
                        backend: hitbox_fn::NoBackend,
                        policy: hitbox_fn::NoPolicy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_call_impl_backend(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;

        quote! {
            impl<P, C> #call_name<hitbox_fn::NoBackend, P, C> {
                pub fn backend<B: hitbox::backend::CacheBackend>(self, backend: B) -> #call_name<hitbox_fn::WithBackend<B>, P, C> {
                    #call_name {
                        args: self.args,
                        backend: hitbox_fn::WithBackend(std::sync::Arc::new(backend)),
                        policy: self.policy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_call_impl_policy(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;

        quote! {
            impl<B, C> #call_name<B, hitbox_fn::NoPolicy, C> {
                pub fn policy(self, policy: hitbox::policy::PolicyConfig) -> #call_name<B, hitbox_fn::WithPolicy, C> {
                    #call_name {
                        args: self.args,
                        backend: self.backend,
                        policy: hitbox_fn::WithPolicy(std::sync::Arc::new(policy)),
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_call_impl_with_context(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;

        quote! {
            impl<B, P> #call_name<B, P, hitbox_fn::NoContext> {
                pub fn with_context(self) -> #call_name<B, P, hitbox_fn::WithContext> {
                    #call_name {
                        args: self.args,
                        backend: self.backend,
                        policy: self.policy,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_cached_call_struct(&self) -> TokenStream {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let args_tuple = self.cached_fn.args_tuple_type();

        quote! {
            pub struct #cached_call_name<'c, B, CM, O, C = hitbox_fn::NoContext> {
                args: hitbox_fn::Args<#args_tuple>,
                cache: &'c hitbox_fn::Cache<B, CM, O>,
                _context: std::marker::PhantomData<C>,
            }
        }
    }

    fn generate_call_impl_cache(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;
        let cached_call_name = &self.cached_fn.cached_call_name;

        quote! {
            impl<B, P, C> #call_name<B, P, C> {
                pub fn cache<CB, CM, O>(self, cache: &hitbox_fn::Cache<CB, CM, O>) -> #cached_call_name<'_, CB, CM, O, C> {
                    #cached_call_name {
                        args: self.args,
                        cache,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_cached_call_impl_with_context(&self) -> TokenStream {
        let cached_call_name = &self.cached_fn.cached_call_name;

        quote! {
            impl<'c, B, CM, O> #cached_call_name<'c, B, CM, O, hitbox_fn::NoContext> {
                pub fn with_context(self) -> #cached_call_name<'c, B, CM, O, hitbox_fn::WithContext> {
                    #cached_call_name {
                        args: self.args,
                        cache: self.cache,
                        _context: std::marker::PhantomData,
                    }
                }
            }
        }
    }

    fn generate_into_future_call_no_context(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        quote! {
            impl<B> std::future::IntoFuture for #call_name<hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::NoContext>
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
            {
                type Output = #return_type;
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

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
        }
    }

    fn generate_into_future_call_with_context(&self) -> TokenStream {
        let call_name = &self.cached_fn.call_name;
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        quote! {
            impl<B> std::future::IntoFuture for #call_name<hitbox_fn::WithBackend<B>, hitbox_fn::WithPolicy, hitbox_fn::WithContext>
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send>>;

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
        }
    }

    fn generate_into_future_cached_no_context(&self) -> TokenStream {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        quote! {
            impl<'c, B, CM, O> std::future::IntoFuture for #cached_call_name<'c, B, CM, O, hitbox_fn::NoContext>
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                CM: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                O: hitbox_core::Offload<'static> + Clone + 'static,
            {
                type Output = #return_type;
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'c>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = std::sync::Arc::clone(self.cache.backend());
                    let policy = std::sync::Arc::clone(self.cache.policy());
                    let concurrency_manager = self.cache.concurrency_manager().clone();
                    let offload = self.cache.offload().clone();
                    let args = self.args;

                    Box::pin(async move {
                        let (result, _ctx) = #execute_name(backend, policy, concurrency_manager, offload, args).await;
                        result
                    })
                }
            }
        }
    }

    fn generate_into_future_cached_with_context(&self) -> TokenStream {
        let cached_call_name = &self.cached_fn.cached_call_name;
        let execute_name = &self.cached_fn.execute_name;
        let return_type = &self.cached_fn.return_type;

        quote! {
            impl<'c, B, CM, O> std::future::IntoFuture for #cached_call_name<'c, B, CM, O, hitbox_fn::WithContext>
            where
                B: hitbox::backend::CacheBackend + Send + Sync + 'static,
                CM: hitbox::concurrency::ConcurrencyManager<#return_type> + Clone + 'static,
                O: hitbox_core::Offload<'static> + Clone + 'static,
            {
                type Output = (#return_type, hitbox::context::CacheContext);
                type IntoFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Self::Output> + Send + 'c>>;

                fn into_future(self) -> Self::IntoFuture {
                    let backend = std::sync::Arc::clone(self.cache.backend());
                    let policy = std::sync::Arc::clone(self.cache.policy());
                    let concurrency_manager = self.cache.concurrency_manager().clone();
                    let offload = self.cache.offload().clone();
                    let args = self.args;

                    Box::pin(async move {
                        #execute_name(backend, policy, concurrency_manager, offload, args).await
                    })
                }
            }
        }
    }

    fn generate_execute_fn(&self) -> TokenStream {
        let impl_name = &self.cached_fn.impl_name;
        let execute_name = &self.cached_fn.execute_name;
        let args_tuple = self.cached_fn.args_tuple_type();
        let return_type = &self.cached_fn.return_type;
        let fn_path = self.cached_fn.fn_path();

        quote! {
            async fn #execute_name<B, CM, O>(
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
        }
    }

    fn generate_public_fn(&self) -> TokenStream {
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

        quote! {
            #vis fn #name(#(#params),*) -> #call_name {
                #call_name::new(hitbox_fn::Args(#args_tuple))
            }
        }
    }
}
