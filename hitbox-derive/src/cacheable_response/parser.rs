//! Parser for CacheableResponse derive macro.

use darling::{FromDeriveInput, FromField, ast::Data};
use syn::{Generics, Ident, Type, Visibility};

/// Field-level attributes.
#[derive(Debug, FromField)]
#[darling(attributes(cacheable_response))]
pub struct FieldAttrs {
    /// Field identifier (None for tuple structs).
    pub ident: Option<Ident>,
    /// Field type.
    pub ty: Type,
    /// Field visibility.
    pub vis: Visibility,
    /// Skip this field from caching (uses Default on reconstruction).
    #[darling(default)]
    pub skip: bool,
}

/// Parsed input for the CacheableResponse derive macro.
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(cacheable_response), supports(struct_named))]
pub struct Source {
    /// The type name.
    pub ident: Ident,
    /// Generic parameters.
    pub generics: Generics,
    /// Struct fields.
    pub data: Data<darling::util::Ignored, FieldAttrs>,
}

impl Source {
    /// Returns true if any field has the `skip` attribute.
    pub fn has_skipped_fields(&self) -> bool {
        match &self.data {
            Data::Struct(fields) => fields.iter().any(|f| f.skip),
            _ => false,
        }
    }

    /// Returns an iterator over all fields.
    pub fn fields(&self) -> impl Iterator<Item = &FieldAttrs> {
        match &self.data {
            Data::Struct(fields) => fields.iter(),
            _ => unreachable!("only struct_named is supported"),
        }
    }
}
