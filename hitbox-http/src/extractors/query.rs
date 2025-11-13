use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

#[derive(Debug)]
pub struct Query<E> {
    inner: E,
    name: String,
}

impl<E> Query<E> {
    pub fn new(inner: E, name: String) -> Self {
        Self { inner, name }
    }
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
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let values = subject
            .parts()
            .uri
            .query()
            .map(crate::query::parse)
            .and_then(|m| m.get(&self.name).map(crate::query::Value::inner))
            .unwrap_or_default();
        let mut parts = self.inner.get(subject).await;
        for value in values {
            parts.push(KeyPart::new(self.name.clone(), Some(value)));
        }
        parts
    }
}
