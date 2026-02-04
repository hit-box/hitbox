use proc_macro2::TokenStream;
use syn::{Error, ItemFn};

use self::generator::Generator;
use self::parser::CachedFn;

mod generator;
mod parser;

pub fn expand(attr: TokenStream, item: ItemFn) -> Result<TokenStream, Error> {
    let cached_fn = CachedFn::new(attr, item)?;
    let generator = Generator::new(&cached_fn);
    Ok(generator.generate())
}
