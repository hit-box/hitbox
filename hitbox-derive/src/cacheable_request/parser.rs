//! Parser for CacheableRequest derive macro.

use darling::FromDeriveInput;
use syn::{Generics, Ident};

/// Parsed input for the CacheableRequest derive macro.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(cacheable_request), supports(struct_any, enum_any))]
pub struct Source {
    /// The type name.
    pub ident: Ident,
    /// Generic parameters.
    pub generics: Generics,
    // Future: field-level validation attributes will be added here
}
