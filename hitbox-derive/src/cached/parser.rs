//! Parser for the `#[cached]` macro.

use convert_case::{Case, Casing};
use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use syn::{
    Error, GenericParam, Ident, ItemFn, LifetimeParam, Pat, PatIdent, PatType, ReturnType,
    Signature, Type, TypeParam, Visibility,
};

/// Wrapper type for parsing a list of identifiers from `skip(a, b, c)` syntax.
#[derive(Debug, Default)]
pub struct SkipList(pub Vec<Ident>);

impl FromMeta for SkipList {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        let mut idents = Vec::new();
        for item in items {
            match item {
                darling::ast::NestedMeta::Meta(syn::Meta::Path(path)) => {
                    if let Some(ident) = path.get_ident() {
                        idents.push(ident.clone());
                    } else {
                        return Err(darling::Error::custom("expected identifier").with_span(path));
                    }
                }
                _ => {
                    return Err(darling::Error::custom("expected identifier"));
                }
            }
        }
        Ok(SkipList(idents))
    }
}

/// Attributes for the `#[cached]` macro, parsed using Darling.
///
/// Supports:
/// - `prefix = "custom_prefix"` - Custom cache key prefix
/// - `skip(param1, param2)` - Parameters to exclude from cache key
#[derive(Debug, Default, FromMeta)]
pub struct CachedAttrs {
    /// Custom prefix for the cache key.
    /// If not specified, the function name is used.
    #[darling(default)]
    pub prefix: Option<String>,

    /// Parameter names to skip from cache key generation.
    #[darling(default)]
    pub skip: SkipList,
}

/// Represents a function argument.
#[derive(Debug)]
pub struct Argument {
    pub name: Ident,
    pub ty: Type,
}

/// Parsed representation of a function annotated with `#[cached]`.
#[derive(Debug)]
pub struct CachedFn {
    pub vis: Visibility,
    pub name: Ident,
    pub impl_name: Ident,
    pub call_name: Ident,
    pub cached_call_name: Ident,
    pub execute_name: Ident,
    pub upstream_name: Ident,
    pub cache_future_name: Ident,
    pub args: Vec<Argument>,
    pub return_type: Type,
    pub body: syn::Block,
    /// Custom prefix for cache key. If None, function name is used.
    pub prefix: Option<String>,
    /// Parameter names to skip from cache key generation.
    pub skip: Vec<String>,
    /// Lifetime parameters from the function signature.
    pub lifetimes: Vec<LifetimeParam>,
    /// Type parameters from the function signature.
    pub type_params: Vec<TypeParam>,
}

impl CachedFn {
    pub fn new(attr: TokenStream, item: ItemFn) -> Result<Self, Error> {
        let attrs = Self::parse_attrs(attr)?;
        let sig = &item.sig;

        if sig.asyncness.is_none() {
            return Err(Error::new_spanned(
                sig,
                "#[cached] can only be applied to async functions",
            ));
        }

        let name = sig.ident.clone();
        let pascal_name = name.to_string().to_case(Case::Pascal);

        let impl_name = Ident::new(&format!("__{}_impl", name), name.span());
        let call_name = Ident::new(&format!("{}Call", pascal_name), name.span());
        let cached_call_name = Ident::new(&format!("{}CallCached", pascal_name), name.span());
        let execute_name = Ident::new(&format!("__execute_cached_{}", name), name.span());
        let upstream_name = Ident::new(&format!("__{}Upstream", pascal_name), name.span());
        let cache_future_name = Ident::new(&format!("__{}CacheFuture", pascal_name), name.span());

        let args = Self::parse_args(sig)?;
        let lifetimes = Self::parse_lifetimes(sig);
        let type_params = Self::parse_type_params(sig);

        let return_type = match &sig.output {
            ReturnType::Default => {
                return Err(Error::new_spanned(
                    sig,
                    "#[cached] functions must have a return type",
                ));
            }
            ReturnType::Type(_, ty) => (**ty).clone(),
        };

        // Convert Ident skip list to String for easier comparison
        let skip: Vec<String> = attrs.skip.0.iter().map(|i| i.to_string()).collect();

        // Validate that all skip names correspond to actual function parameters
        let arg_names = args.iter().map(|a| a.name.to_string()).collect::<Vec<_>>();
        for skip_ident in &attrs.skip.0 {
            if !arg_names.contains(&skip_ident.to_string()) {
                return Err(Error::new_spanned(
                    skip_ident,
                    format!(
                        "unknown parameter `{}` in skip list; available parameters: {}",
                        skip_ident,
                        arg_names.join(", "),
                    ),
                ));
            }
        }

        Ok(Self {
            vis: item.vis,
            name,
            impl_name,
            call_name,
            cached_call_name,
            execute_name,
            upstream_name,
            cache_future_name,
            args,
            return_type,
            body: (*item.block).clone(),
            prefix: attrs.prefix,
            skip,
            lifetimes,
            type_params,
        })
    }

    /// Parse macro attributes using Darling.
    fn parse_attrs(attr: TokenStream) -> Result<CachedAttrs, Error> {
        if attr.is_empty() {
            return Ok(CachedAttrs::default());
        }

        let meta_list = NestedMeta::parse_meta_list(attr)?;
        CachedAttrs::from_list(&meta_list).map_err(|e| e.into())
    }

    /// Parse function arguments from signature.
    fn parse_args(sig: &Signature) -> Result<Vec<Argument>, Error> {
        let mut args = Vec::new();

        for arg in &sig.inputs {
            match arg {
                syn::FnArg::Receiver(_) => {
                    return Err(Error::new_spanned(
                        arg,
                        "#[cached] cannot be applied to methods with self",
                    ));
                }
                syn::FnArg::Typed(PatType { pat, ty, .. }) => {
                    let name = match pat.as_ref() {
                        Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                        _ => {
                            return Err(Error::new_spanned(
                                pat,
                                "Expected a simple identifier pattern",
                            ));
                        }
                    };
                    args.push(Argument {
                        name,
                        ty: (**ty).clone(),
                    });
                }
            }
        }

        Ok(args)
    }

    /// Extract lifetime parameters from function signature.
    fn parse_lifetimes(sig: &Signature) -> Vec<LifetimeParam> {
        sig.generics
            .params
            .iter()
            .filter_map(|param| {
                if let GenericParam::Lifetime(lt) = param {
                    Some(lt.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract type parameters from function signature.
    fn parse_type_params(sig: &Signature) -> Vec<TypeParam> {
        sig.generics
            .params
            .iter()
            .filter_map(|param| {
                if let GenericParam::Type(tp) = param {
                    Some(tp.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    // =========================================================================
    // Generic helpers (lifetimes + type parameters)
    // =========================================================================

    /// Returns true if the function has any generic parameters (lifetimes or type params).
    pub fn has_generics(&self) -> bool {
        !self.lifetimes.is_empty() || !self.type_params.is_empty()
    }

    /// Returns true if the function has 2+ lifetime parameters,
    /// requiring a synthetic `'__hitbox` lifetime to represent their minimum.
    pub fn needs_synthetic_lifetime(&self) -> bool {
        self.lifetimes.len() >= 2
    }

    /// Returns the synthetic `'__hitbox` lifetime token.
    ///
    /// Used as a single "minimum" lifetime when a function has 2+ lifetime
    /// parameters. All user lifetimes are bounded by it (`'a: '__hitbox`),
    /// so the compiler infers it as the shortest.
    pub fn synthetic_lifetime(&self) -> syn::Lifetime {
        syn::Lifetime::new("'__hitbox", proc_macro2::Span::call_site())
    }

    /// Returns the user's original generic parameters without synthetic lifetime.
    /// Used only for the internal impl function (the actual async fn body).
    /// Example: `'a, 'b, T: Clone` (without angle brackets)
    pub fn user_generic_params(&self) -> TokenStream {
        let lifetimes = &self.lifetimes;
        let type_params = &self.type_params;

        match (self.lifetimes.is_empty(), self.type_params.is_empty()) {
            (true, true) => quote::quote! {},
            (false, true) => quote::quote! { #(#lifetimes),* },
            (true, false) => quote::quote! { #(#type_params),* },
            (false, false) => quote::quote! { #(#lifetimes),*, #(#type_params),* },
        }
    }

    /// Returns all generic parameters for declarations (impl blocks, struct definitions).
    ///
    /// For 2+ lifetimes, prepends a synthetic `'__hitbox` lifetime and bounds all
    /// user lifetimes to outlive it. The compiler infers `'__hitbox` as the shortest.
    ///
    /// - 0 lifetimes: `T: Clone` (type params only)
    /// - 1 lifetime: `'a, T: Clone` (original)
    /// - 2+ lifetimes: `'__hitbox, 'a: '__hitbox, 'b: '__hitbox, T: Clone`
    pub fn generic_params(&self) -> TokenStream {
        let type_params = &self.type_params;

        if self.needs_synthetic_lifetime() {
            let synthetic = self.synthetic_lifetime();
            let bounded: Vec<_> = self
                .lifetimes
                .iter()
                .map(|lt| {
                    let lifetime = &lt.lifetime;
                    let bounds = &lt.bounds;
                    if bounds.is_empty() {
                        quote::quote! { #lifetime: #synthetic }
                    } else {
                        quote::quote! { #lifetime: #bounds + #synthetic }
                    }
                })
                .collect();
            if type_params.is_empty() {
                quote::quote! { #synthetic, #(#bounded),* }
            } else {
                quote::quote! { #synthetic, #(#bounded),*, #(#type_params),* }
            }
        } else {
            let lifetimes = &self.lifetimes;
            match (self.lifetimes.is_empty(), self.type_params.is_empty()) {
                (true, true) => quote::quote! {},
                (false, true) => quote::quote! { #(#lifetimes),* },
                (true, false) => quote::quote! { #(#type_params),* },
                (false, false) => quote::quote! { #(#lifetimes),*, #(#type_params),* },
            }
        }
    }

    /// Returns all generic arguments for type applications.
    ///
    /// - 0 lifetimes: `T` (type idents only)
    /// - 1 lifetime: `'a, T`
    /// - 2+ lifetimes: `'__hitbox, 'a, 'b, T`
    pub fn generic_args(&self) -> TokenStream {
        let lifetime_idents: Vec<_> = self.lifetimes.iter().map(|lt| &lt.lifetime).collect();
        let type_idents: Vec<_> = self.type_params.iter().map(|tp| &tp.ident).collect();

        if self.needs_synthetic_lifetime() {
            let synthetic = self.synthetic_lifetime();
            if type_idents.is_empty() {
                quote::quote! { #synthetic, #(#lifetime_idents),* }
            } else {
                quote::quote! { #synthetic, #(#lifetime_idents),*, #(#type_idents),* }
            }
        } else {
            match (lifetime_idents.is_empty(), type_idents.is_empty()) {
                (true, true) => quote::quote! {},
                (false, true) => quote::quote! { #(#lifetime_idents),* },
                (true, false) => quote::quote! { #(#type_idents),* },
                (false, false) => quote::quote! { #(#lifetime_idents),*, #(#type_idents),* },
            }
        }
    }

    /// Returns the PhantomData type for struct context fields.
    ///
    /// When a synthetic lifetime exists, includes it in the phantom to "use" it:
    /// - No synthetic: `std::marker::PhantomData<C>`
    /// - With synthetic: `std::marker::PhantomData<(&'__hitbox (), C)>`
    pub fn phantom_context_type(&self) -> TokenStream {
        if self.needs_synthetic_lifetime() {
            let synthetic = self.synthetic_lifetime();
            quote::quote! { std::marker::PhantomData<(&#synthetic (), C)> }
        } else {
            quote::quote! { std::marker::PhantomData<C> }
        }
    }

    // =========================================================================
    // Argument tuple helpers
    // =========================================================================

    /// Returns the tuple type with each argument wrapped in `Arg<T>` or `Skipped<T>`.
    /// Example: `(Skipped<String>, Arg<i64>)`
    pub fn args_tuple_type(&self) -> TokenStream {
        let types: Vec<_> = self
            .args
            .iter()
            .map(|a| {
                let ty = &a.ty;
                let is_skipped = self.skip.contains(&a.name.to_string());
                if is_skipped {
                    quote::quote! { hitbox_fn::Skipped<#ty> }
                } else {
                    quote::quote! { hitbox_fn::Arg<#ty> }
                }
            })
            .collect();
        if types.len() == 1 {
            let ty = &types[0];
            quote::quote! { (#ty,) }
        } else {
            quote::quote! { (#(#types),*) }
        }
    }

    /// Returns the tuple expression with each argument wrapped in `Arg::new()` or `Skipped::new()`.
    /// Example: `(Skipped::new(request_id), Arg::new("value", value))`
    pub fn args_tuple_expr(&self) -> TokenStream {
        let exprs: Vec<_> = self
            .args
            .iter()
            .map(|a| {
                let name = &a.name;
                let is_skipped = self.skip.contains(&a.name.to_string());
                if is_skipped {
                    quote::quote! { hitbox_fn::Skipped::new(#name) }
                } else {
                    let name_str = name.to_string();
                    quote::quote! { hitbox_fn::Arg::new(#name_str, #name) }
                }
            })
            .collect();
        if exprs.len() == 1 {
            let expr = &exprs[0];
            quote::quote! { (#expr,) }
        } else {
            quote::quote! { (#(#exprs),*) }
        }
    }

    /// Returns the destructuring pattern for Args tuple.
    /// Example: `(__arg0, __arg1)`
    pub fn args_destructure_pattern(&self) -> TokenStream {
        let patterns: Vec<_> = self
            .args
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let name = Ident::new(&format!("__arg{}", i), proc_macro2::Span::call_site());
                quote::quote! { #name }
            })
            .collect();
        if patterns.len() == 1 {
            let pat = &patterns[0];
            quote::quote! { (#pat,) }
        } else {
            quote::quote! { (#(#patterns),*) }
        }
    }

    /// Returns let bindings to extract values from Arg wrappers.
    /// Example: `let request_id = __arg0.into_value(); let value = __arg1.into_value();`
    pub fn args_extract_values(&self) -> TokenStream {
        let bindings: Vec<_> = self
            .args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let arg_name = Ident::new(&format!("__arg{}", i), proc_macro2::Span::call_site());
                let var_name = &a.name;
                quote::quote! { let #var_name = #arg_name.into_value(); }
            })
            .collect();
        quote::quote! { #(#bindings)* }
    }

    /// Returns the cache key prefix.
    /// Uses custom prefix if specified, otherwise falls back to function name.
    pub fn fn_path(&self) -> String {
        self.prefix.clone().unwrap_or_else(|| self.name.to_string())
    }

    /// Returns the expression to instantiate the upstream struct.
    ///
    /// - For functions with type params: `UpstreamName::<T>(std::marker::PhantomData)`
    /// - For synthetic lifetime (no types): `UpstreamName(std::marker::PhantomData)`
    /// - Otherwise (unit struct): `UpstreamName`
    pub fn upstream_instance(&self) -> TokenStream {
        let upstream_name = &self.upstream_name;
        if !self.type_params.is_empty() {
            // Type params need turbofish; lifetimes (including '__hitbox) are inferred.
            let type_idents: Vec<_> = self.type_params.iter().map(|tp| &tp.ident).collect();
            quote::quote! { #upstream_name::<#( #type_idents ),*>(std::marker::PhantomData) }
        } else if self.needs_synthetic_lifetime() {
            // Synthetic lifetime on struct — no turbofish, lifetime is inferred.
            quote::quote! { #upstream_name(std::marker::PhantomData) }
        } else {
            // Unit struct (lifetimes, if any, are only on the impl block).
            quote::quote! { #upstream_name }
        }
    }

    /// Returns the offload lifetime for bounds.
    ///
    /// - 0 lifetimes: `'static`
    /// - 1 lifetime: the user's lifetime directly (e.g., `'a`)
    /// - 2+ lifetimes: synthetic `'__hitbox` (bounded by all user lifetimes)
    pub fn offload_lifetime(&self) -> TokenStream {
        match self.lifetimes.len() {
            0 => quote::quote! { 'static },
            1 => {
                let lt = &self.lifetimes[0].lifetime;
                quote::quote! { #lt }
            }
            _ => {
                let synthetic = self.synthetic_lifetime();
                quote::quote! { #synthetic }
            }
        }
    }
}
