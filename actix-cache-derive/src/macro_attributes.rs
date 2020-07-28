use syn;
use syn::{Attribute, NestedMeta};

fn parse_attribute(attr: &Attribute, method: &str) -> Option<u32> {
    if attr.path.is_ident(method) {
        let meta = &attr.parse_meta();
        match meta {
            Ok(syn::Meta::List(value)) => {
                let nested = value.nested.first()?;
                let result = match nested {
                    NestedMeta::Lit(syn::Lit::Int(value)) => Some(value.base10_parse().ok()?),
                    _ => panic!("Parameter for macro {macro} should be u32", macro=method),
                };
                result.into()
            }
            _ => panic!("{macro} macro should have a parameter of type u32", macro=method),
        }
    } else {
        None
    }
}

pub fn find_attribute(ast: &syn::DeriveInput, method: &str) -> Option<u32> {
    ast.attrs
        .iter()
        .find_map(|attr| parse_attribute(attr, method))
}
