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

    pub fn args_tuple_type(&self) -> TokenStream {
        let types: Vec<_> = self.args.iter().map(|a| &a.ty).collect();
        if types.len() == 1 {
            let ty = &types[0];
            quote::quote! { (#ty,) }
        } else {
            quote::quote! { (#(#types),*) }
        }
    }

    pub fn args_tuple_expr(&self) -> TokenStream {
        let names: Vec<_> = self.args.iter().map(|a| &a.name).collect();
        if names.len() == 1 {
            let name = &names[0];
            quote::quote! { (#name,) }
        } else {
            quote::quote! { (#(#names),*) }
        }
    }

    pub fn args_destructure(&self) -> TokenStream {
        let names: Vec<_> = self.args.iter().map(|a| &a.name).collect();
        if names.len() == 1 {
            let name = &names[0];
            quote::quote! { (#name,) }
        } else {
            quote::quote! { (#(#names),*) }
        }
    }

    /// Returns the cache key prefix.
    /// Uses custom prefix if specified, otherwise falls back to function name.
    pub fn fn_path(&self) -> String {
        self.prefix.clone().unwrap_or_else(|| self.name.to_string())
    }
}
