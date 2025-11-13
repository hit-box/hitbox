use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use http::HeaderValue;

use crate::CacheableHttpRequest;

#[derive(Debug)]
pub struct Header<E> {
    inner: E,
    name: String,
}

pub trait HeaderExtractor: Sized {
    fn header(self, name: String) -> Header<Self>;
}

impl<E> HeaderExtractor for E
where
    E: Extractor,
{
    fn header(self, name: String) -> Header<Self> {
        Header { inner: self, name }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Header<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let value = subject
            .parts()
            .headers
            .get(self.name.as_str())
            .map(HeaderValue::to_str)
            .transpose()
            .ok()
            .flatten()
            .map(str::to_string);
        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new(self.name.clone(), value));
        parts
    }
}
