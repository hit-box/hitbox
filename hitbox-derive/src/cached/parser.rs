//! Parser for the `#[cached]` macro.

use convert_case::{Case, Casing};
use darling::{FromMeta, ast::NestedMeta};
use proc_macro2::TokenStream;
use syn::{
    Error, GenericParam, Ident, ItemFn, Pat, PatIdent, PatType, ReturnType, Signature, Type,
    TypeParam, Visibility,
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
    pub args: Vec<Argument>,
    pub return_type: Type,
    pub body: syn::Block,
    /// Custom prefix for cache key. If None, function name is used.
    pub prefix: Option<String>,
    /// Parameter names to skip from cache key generation.
    pub skip: Vec<String>,
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

        // Reject lifetime parameters — borrowed args are not yet supported (see #206)
        for param in &sig.generics.params {
            if let GenericParam::Lifetime(lt) = param {
                return Err(Error::new_spanned(
                    lt,
                    "#[cached] does not support lifetime parameters; borrowed arguments are planned for 0.3 (see #206)",
                ));
            }
        }

        let args = Self::parse_args(sig)?;
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
            args,
            return_type,
            body: (*item.block).clone(),
            prefix: attrs.prefix,
            skip,
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
    // Generic helpers (type parameters only; lifetimes rejected in parser)
    // =========================================================================

    /// Returns true if the function has any type parameters.
    pub fn has_generics(&self) -> bool {
        !self.type_params.is_empty()
    }

    /// Returns type parameters with bounds for declarations.
    /// Example: `T: Clone, U: Debug`
    pub fn generic_params(&self) -> TokenStream {
        let type_params = &self.type_params;
        quote::quote! { #(#type_params),* }
    }

    /// Returns type parameter idents without bounds for type applications.
    /// Example: `T, U`
    pub fn generic_args(&self) -> TokenStream {
        let type_idents: Vec<_> = self.type_params.iter().map(|tp| &tp.ident).collect();
        quote::quote! { #(#type_idents),* }
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
}
