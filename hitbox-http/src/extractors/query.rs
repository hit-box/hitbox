use async_trait::async_trait;
use hitbox::cache::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

pub struct Query<E> {
    inner: E,
    name: String,
}

pub trait QueryExtractor: Sized {
    fn query(self, name: String) -> Query<Self>;
}

impl<E> QueryExtractor for E
where
    E: Extractor,
{
    fn query(self, name: String) -> Query<Self> {
        Query { inner: self, name }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Query<E>
where
    ReqBody: Send + 'static,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let values = subject
            .parts()
            .uri
            .query()
            .map(hitbox_qs::parse)
            .map(|m| m.get(&self.name).map(hitbox_qs::Value::inner))
            .flatten()
            .unwrap_or_default();
        let mut parts = self.inner.get(subject).await;
        for value in values {
        parts.push(KeyPart {
            key: self.name.clone(),
            value: Some(value),
        });}
        parts
    }
}
