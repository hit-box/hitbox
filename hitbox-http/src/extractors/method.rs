use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

pub struct Method<E> {
    inner: E,
}

pub trait MethodExtractor: Sized {
    fn method(self) -> Method<Self>;
}

impl<E> MethodExtractor for E
where
    E: Extractor,
{
    fn method(self) -> Method<Self> {
        Method { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Method<E>
where
    ReqBody: Send + 'static,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let method = subject.parts().method.to_string();
        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new("method".to_owned(), Some(method)));
        parts
    }
}
