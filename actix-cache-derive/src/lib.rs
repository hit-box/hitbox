extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

fn impl_cacheable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl Cacheable for #name {
            fn cache_key(&self) -> String {
                serde_urlencoded::to_string(self).unwrap()
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(Cacheable)]
pub fn cacheable_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_cacheable_macro(&ast)
}
