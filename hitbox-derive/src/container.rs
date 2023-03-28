use quote::ToTokens;

const CACHE_TTL: &str = "cache_ttl";
const CACHE_STALE_TTL: &str = "cache_stale_ttl";
const CACHE_VERSION: &str = "cache_version";
const HITBOX: &str = "hitbox";

fn parse_lit_to_u32(expr: &syn::Expr, attr_name: &str) -> syn::Result<u32> {
    if let syn::Expr::Lit(expr) = expr {
        if let syn::Lit::Int(lit) = &expr.lit {
            return lit
                .base10_parse::<u32>()
                .map_err(|e| syn::Error::new_spanned(lit, e));
        }
    }
    Err(syn::Error::new_spanned(
        expr,
        format!("Expected hitbox {attr_name} attribute should be u32"),
    ))
}

pub struct Container {
    pub cache_ttl: Option<u32>,
    pub cache_stale_ttl: Option<u32>,
    pub cache_version: Option<u32>,
}

impl Container {
    pub fn from_ast(input: &syn::DeriveInput) -> syn::Result<Self> {
        let mut ttl = None;
        let mut stale_ttl = None;
        let mut version = None;

        let items = input
            .attrs
            .iter()
            .map(|attr| {
                if !attr.path().is_ident(HITBOX) {
                    return Ok(Vec::new());
                }

                let nested = attr.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                );

                match nested {
                    Ok(nested) => Ok(nested.into_iter().collect()),
                    Err(err) => Err(err),
                }
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten();

        for meta_item in items {
            match &meta_item {
                // Parse `#[hitbox(cache_ttl = 42)]`
                syn::Meta::NameValue(m) if m.path.is_ident(CACHE_TTL) => {
                    ttl = Some(parse_lit_to_u32(&m.value, CACHE_TTL)?);
                }

                // Parse `#[hitbox(cache_stale_ttl = 42)]`
                syn::Meta::NameValue(m) if m.path.is_ident(CACHE_STALE_TTL) => {
                    stale_ttl = Some(parse_lit_to_u32(&m.value, CACHE_STALE_TTL)?);
                }

                // Parse `#[hitbox(cache_version = 42)]`
                syn::Meta::NameValue(m) if m.path.is_ident(CACHE_VERSION) => {
                    version = Some(parse_lit_to_u32(&m.value, CACHE_VERSION)?);
                }

                // Throw error on unknown attribute
                meta_value => {
                    let path = meta_value
                        .path()
                        .into_token_stream()
                        .to_string()
                        .replace(' ', "");
                    return Err(syn::Error::new_spanned(
                        meta_value.path(),
                        format!("Unknown hitbox container attribute `{path}`"),
                    ));
                }
            }
        }

        Ok(Container {
            cache_ttl: ttl,
            cache_stale_ttl: stale_ttl,
            cache_version: version,
        })
    }
}
