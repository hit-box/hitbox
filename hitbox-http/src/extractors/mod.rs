use std::marker::PhantomData;

use async_trait::async_trait;
use hitbox::{Extractor, KeyParts};

use crate::CacheableHttpRequest;

pub mod header;
pub mod method;
pub mod path;
pub mod query;

pub struct NeutralExtractor<ReqBody> {
    _res: PhantomData<fn(ReqBody) -> ReqBody>,
}

impl<ResBody> NeutralExtractor<ResBody> {
    pub fn new() -> Self {
        NeutralExtractor { _res: PhantomData }
    }
}

#[async_trait]
impl<ResBody> Extractor for NeutralExtractor<ResBody>
where
    ResBody: Send + 'static,
{
    type Subject = CacheableHttpRequest<ResBody>;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        KeyParts::new(subject)
    }
}

impl<ResBody> Default for NeutralExtractor<ResBody> {
    fn default() -> Self {
        Self::new()
    }
}
