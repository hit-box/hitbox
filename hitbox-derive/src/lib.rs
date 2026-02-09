#![doc = include_str!("../README.md")]

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, ItemFn, parse_macro_input};

mod cacheable_request;
mod cacheable_response;
mod cached;
mod generator;
mod parser;

use crate::generator::Generator;
use crate::parser::Source;

/// Derive macro for `KeyExtract` trait.
///
/// Generates an implementation of `KeyExtract` that creates key parts from struct fields.
///
/// # Attributes
///
/// - `#[key_extract(name = "...")]` - Override the key part name (default: field name)
/// - `#[key_extract(skip)]` - Skip this field in key generation
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::KeyExtract;
///
/// #[derive(KeyExtract)]
/// struct UserRequest {
///     user_id: u64,
///     #[key_extract(skip)]
///     password: String,
/// }
/// ```
#[proc_macro_derive(KeyExtract, attributes(key_extract))]
pub fn derive_key_extract(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_impl(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> Result<TokenStream, Error> {
    let source = Source::from_derive_input(input)?;
    let fields = source.struct_fields()?.collect::<Vec<_>>();
    let generator = Generator::new(&source, &fields);
    Ok(quote! { #generator })
}

/// Attribute macro for caching async functions.
///
/// Transforms an async function to return a builder that can be configured with
/// backend and policy before execution.
///
/// # Attributes
///
/// - `prefix = "..."` - Custom prefix for the cache key (default: function name)
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::cached;
///
/// #[cached]
/// async fn fetch_user(id: UserId, tenant: TenantId) -> Result<User, Error> {
///     // expensive operation
/// }
///
/// // With custom prefix
/// #[cached(prefix = "user_data")]
/// async fn fetch_user(id: UserId) -> Result<User, Error> {
///     // expensive operation
/// }
///
/// // Zero-argument function
/// #[cached(prefix = "app_config")]
/// async fn get_config() -> Result<Config, Error> {
///     // expensive operation
/// }
///
/// // Usage with pre-configured cache
/// let cache = Cache::builder()
///     .backend(backend)
///     .policy(policy)
///     .build();
///
/// let user = fetch_user(UserId(42), TenantId("acme".into()))
///     .cache(&cache)
///     .await?;
///
/// // Usage with context
/// let (user, ctx) = fetch_user(UserId(42), TenantId("acme".into()))
///     .cache(&cache)
///     .with_context()
///     .await;
/// println!("Cache status: {:?}", ctx.status);
///
/// // Usage with inline configuration
/// let user = fetch_user(UserId(42), TenantId("acme".into()))
///     .backend(backend)
///     .policy(policy)
///     .await?;
/// ```
#[proc_macro_attribute]
pub fn cached(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    match cached::expand(attr.into(), item) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derive macro for `CacheableResponse` trait.
///
/// Generates an implementation of `CacheableResponse` where the type is cached as itself.
/// The type must implement `Clone + Serialize + DeserializeOwned + Send + 'static`.
///
/// # Example
///
/// ```ignore
/// use hitbox_derive::CacheableResponse;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Serialize, Deserialize, CacheableResponse)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// // Now User can be used as a return type in cached functions:
/// #[cached]
/// async fn fetch_user(id: u64) -> Result<User, Error> {
///     // ...
/// }
/// ```
#[proc_macro_derive(CacheableResponse, attributes(cacheable_response))]
pub fn derive_cacheable_response(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match cacheable_response::expand(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derive macro for `CacheableRequest` trait.
///
/// Generates an implementation of `CacheableRequest` with standard cache policy logic.
/// The type can then participate in the hitbox caching pipeline.
///
/// # Example
///
/// ```ignore
/// use hitbox_derive::CacheableRequest;
///
/// #[derive(CacheableRequest)]
/// struct SearchRequest {
///     query: String,
///     page: u32,
/// }
/// ```
#[proc_macro_derive(CacheableRequest, attributes(cacheable_request))]
pub fn derive_cacheable_request(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match cacheable_request::expand(&input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
