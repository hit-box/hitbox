use darling::{FromDeriveInput, FromField, ast::Data, util::Ignored};
use syn::{Error, Ident};

#[derive(Debug, FromField)]
#[darling(attributes(key_extract))]
pub(crate) struct FieldAttributes {
    pub(crate) ident: Option<Ident>,
    #[darling(default)]
    pub(crate) skip: bool,
    #[darling(default)]
    pub(crate) name: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Field<'a> {
    pub(crate) attributes: &'a FieldAttributes,
    pub(crate) index: usize,
}

impl<'a> Field<'a> {
    pub(crate) fn new(_source: &'a Source, attributes: &'a FieldAttributes, index: usize) -> Self {
        Self { attributes, index }
    }

    pub(crate) fn key_name(&self) -> String {
        self.attributes
            .name
            .clone()
            .or_else(|| self.attributes.ident.as_ref().map(|i| i.to_string()))
            .unwrap_or_else(|| self.index.to_string())
    }

    pub(crate) fn is_skipped(&self) -> bool {
        self.attributes.skip
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(key_extract), supports(struct_any))]
pub(crate) struct Source {
    pub(crate) ident: Ident,
    pub(crate) data: Data<Ignored, FieldAttributes>,
}

impl Source {
    pub(crate) fn struct_fields(&self) -> Result<impl Iterator<Item = Field<'_>>, Error> {
        match &self.data {
            Data::Struct(fields) => Ok(fields
                .iter()
                .enumerate()
                .map(|(index, attributes)| Field::new(self, attributes, index))),
            _ => Err(Error::new(
                self.ident.span(),
                "KeyExtract can only be derived for structs",
            )),
        }
    }
}
