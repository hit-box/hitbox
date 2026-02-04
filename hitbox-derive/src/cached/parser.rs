use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use syn::parse::Parser;
use syn::{Error, Ident, ItemFn, Pat, PatIdent, PatType, ReturnType, Signature, Type, Visibility};

/// Attributes for the `#[cached]` macro.
#[derive(Debug, Default)]
pub struct CachedAttrs {
    /// Custom prefix for the cache key.
    /// If not specified, the function name is used.
    pub prefix: Option<String>,
}

#[derive(Debug)]
pub struct Argument {
    pub name: Ident,
    pub ty: Type,
    /// Skip this parameter from cache key generation.
    pub skip: bool,
}

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

        let args = Self::parse_args(sig)?;

        let return_type = match &sig.output {
            ReturnType::Default => {
                return Err(Error::new_spanned(
                    sig,
                    "#[cached] functions must have a return type",
                ));
            }
            ReturnType::Type(_, ty) => (**ty).clone(),
        };

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
        })
    }

    fn parse_attrs(attr: TokenStream) -> Result<CachedAttrs, Error> {
        if attr.is_empty() {
            return Ok(CachedAttrs::default());
        }

        // Parse as a list of meta items (e.g., `prefix = "value"`)
        let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
        let metas = parser.parse2(attr)?;

        let mut attrs = CachedAttrs::default();
        for meta in metas {
            match &meta {
                syn::Meta::NameValue(nv) if nv.path.is_ident("prefix") => {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    {
                        attrs.prefix = Some(s.value());
                    } else {
                        return Err(Error::new_spanned(&nv.value, "expected string literal"));
                    }
                }
                _ => {
                    return Err(Error::new_spanned(
                        &meta,
                        format!("unknown attribute: {}", quote::quote!(#meta)),
                    ));
                }
            }
        }

        Ok(attrs)
    }

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
                syn::FnArg::Typed(PatType { pat, ty, attrs, .. }) => {
                    let name = match pat.as_ref() {
                        Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                        _ => {
                            return Err(Error::new_spanned(
                                pat,
                                "Expected a simple identifier pattern",
                            ));
                        }
                    };
                    let skip = Self::parse_key_extract_skip(attrs)?;
                    args.push(Argument {
                        name,
                        ty: (**ty).clone(),
                        skip,
                    });
                }
            }
        }

        Ok(args)
    }

    fn parse_key_extract_skip(attrs: &[syn::Attribute]) -> Result<bool, Error> {
        for attr in attrs {
            if !attr.path().is_ident("key_extract") {
                continue;
            }

            let mut skip = false;
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                    Ok(())
                } else {
                    Err(meta.error("expected `skip`"))
                }
            })?;

            if skip {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Returns the tuple type with each argument wrapped in `Arg<T>`.
    /// Example: `(Arg<String>, Arg<i64>)`
    pub fn args_tuple_type(&self) -> TokenStream {
        let types: Vec<_> = self
            .args
            .iter()
            .map(|a| {
                let ty = &a.ty;
                quote::quote! { hitbox_fn::Arg<#ty> }
            })
            .collect();
        if types.len() == 1 {
            let ty = &types[0];
            quote::quote! { (#ty,) }
        } else {
            quote::quote! { (#(#types),*) }
        }
    }

    /// Returns the tuple expression with each argument wrapped in `Arg::new()` or `Arg::skipped()`.
    /// Example: `(Arg::skipped(request_id), Arg::new(value))`
    pub fn args_tuple_expr(&self) -> TokenStream {
        let exprs: Vec<_> = self
            .args
            .iter()
            .map(|a| {
                let name = &a.name;
                if a.skip {
                    quote::quote! { hitbox_fn::Arg::skipped(#name) }
                } else {
                    quote::quote! { hitbox_fn::Arg::new(#name) }
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
