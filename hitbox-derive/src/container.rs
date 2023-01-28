const CACHE_TTL: &str = "cache_ttl";
const CACHE_STALE_TTL: &str = "cache_stale_ttl";
const CACHE_VERSION: &str = "cache_version";

use quote::ToTokens;

fn get_lit_int<'a>(lit: &'a syn::Lit, attr_name: &'a str) -> Result<&'a syn::LitInt, syn::Error> {
    if let syn::Lit::Int(lit) = lit {
        Ok(lit)
    } else {
        Err(syn::Error::new_spanned(
            lit,
            format!("Expected hitbox {} attribute should be u32", attr_name),
        ))
    }
}

fn parse_u32(lit: &syn::LitInt) -> syn::Result<u32> {
    lit.base10_parse::<u32>()
        .map_err(|e| syn::Error::new_spanned(lit, e))
}

pub struct Container {
    pub ttl: Option<u32>,
    pub stale_ttl: Option<u32>,
    pub version: Option<u32>,
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
                if !attr.path.is_ident("hitbox") {
                    return Ok(Vec::new());
                }

                match attr.parse_meta() {
                    Ok(syn::Meta::List(meta)) => Ok(meta.nested.into_iter().collect()),
                    Ok(other) => Err(syn::Error::new_spanned(other, "expected #[hitbox(...)]")),
                    Err(err) => Err(err),
                }
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten();

        for meta_item in items {
            match &meta_item {
                // Parse `#[hitbox(cache_ttl = 42)]`
                syn::NestedMeta::Meta(syn::Meta::NameValue(m)) if m.path.is_ident(CACHE_TTL) => {
                    let lit_ttl = get_lit_int(&m.lit, CACHE_TTL)?;
                    ttl = Some(parse_u32(lit_ttl)?)
                }

                // Parse `#[hitbox(cache_stale_ttl = 42)]`
                syn::NestedMeta::Meta(syn::Meta::NameValue(m))
                    if m.path.is_ident(CACHE_STALE_TTL) =>
                {
                    let lit_stale_ttl = get_lit_int(&m.lit, CACHE_STALE_TTL)?;
                    stale_ttl = Some(parse_u32(lit_stale_ttl)?)
                }

                // Parse `#[hitbox(cache_version = 42)]`
                syn::NestedMeta::Meta(syn::Meta::NameValue(m))
                    if m.path.is_ident(CACHE_VERSION) =>
                {
                    let lit_version = get_lit_int(&m.lit, CACHE_VERSION)?;
                    version = Some(parse_u32(lit_version)?)
                }

                // Throw error on unknown attribute
                syn::NestedMeta::Meta(m) => {
                    let path = m.path().into_token_stream().to_string().replace(' ', "");
                    return Err(syn::Error::new_spanned(
                        m.path(),
                        format!("Unknown hitbox container attribute `{}`", path),
                    ));
                }

                // Throw error on other lit types
                lit => {
                    return Err(syn::Error::new_spanned(
                        lit,
                        "Unexpected literal in hitbox container attribute",
                    ));
                }
            }
        }

        Ok(Container {
            ttl,
            stale_ttl,
            version,
        })
    }
}
