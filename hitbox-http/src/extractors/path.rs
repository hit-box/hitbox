use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

#[derive(Debug)]
pub struct Path<E> {
    inner: E,
    resource: ResourceDef,
}

impl<E> Path<E> {
    pub fn new(inner: E, resource: String) -> Self {
        Self {
            inner,
            resource: ResourceDef::from(resource),
        }
    }
}

pub trait PathExtractor: Sized {
    fn path(self, resource: &str) -> Path<Self>;
}

impl<E> PathExtractor for E
where
    E: Extractor,
{
    fn path(self, resource: &str) -> Path<Self> {
        Path {
            inner: self,
            resource: ResourceDef::from(resource),
        }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Path<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let mut path = actix_router::Path::new(subject.parts().uri.path());
        self.resource.capture_match_info(&mut path);
        let mut matched_parts = path
            .iter()
            .map(|(key, value)| KeyPart::new(key.to_owned(), Some(value.to_owned())))
            .collect::<Vec<_>>();
        let mut parts = self.inner.get(subject).await;
        parts.append(&mut matched_parts);
        parts
    }
}
