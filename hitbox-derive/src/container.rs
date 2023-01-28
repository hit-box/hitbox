use quote::ToTokens;

const CACHE_TTL: &str = "cache_ttl";
const CACHE_STALE_TTL: &str = "cache_stale_ttl";
const CACHE_VERSION: &str = "cache_version";

fn parse_lit_to_u32(lit: &syn::Lit, attr_name: &str) -> syn::Result<u32> {
    match lit {
        syn::Lit::Int(lit) => lit
            .base10_parse::<u32>()
            .map_err(|e| syn::Error::new_spanned(lit, e)),
        _ => Err(syn::Error::new_spanned(
            lit,
            format!("Expected hitbox {attr_name} attribute should be u32"),
        )),
    }
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
                    ttl = Some(parse_lit_to_u32(&m.lit, CACHE_TTL)?);
                }

                // Parse `#[hitbox(cache_stale_ttl = 42)]`
                syn::NestedMeta::Meta(syn::Meta::NameValue(m))
                    if m.path.is_ident(CACHE_STALE_TTL) =>
                {
                    stale_ttl = Some(parse_lit_to_u32(&m.lit, CACHE_STALE_TTL)?);
                }

                // Parse `#[hitbox(cache_version = 42)]`
                syn::NestedMeta::Meta(syn::Meta::NameValue(m))
                    if m.path.is_ident(CACHE_VERSION) =>
                {
                    version = Some(parse_lit_to_u32(&m.lit, CACHE_VERSION)?);
                }

                // Throw error on unknown attribute
                syn::NestedMeta::Meta(m) => {
                    let path = m.path().into_token_stream().to_string().replace(' ', "");
                    return Err(syn::Error::new_spanned(
                        m.path(),
                        format!("Unknown hitbox container attribute `{path}`"),
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
            cache_ttl: ttl,
            cache_stale_ttl: stale_ttl,
            cache_version: version,
        })
    }
}
