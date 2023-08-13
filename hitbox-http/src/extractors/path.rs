use actix_router::{ResourceDef, ResourcePath};
use async_trait::async_trait;
use hitbox::cache::{CacheableRequest, Extractor, KeyPart, KeyParts};
use http::HeaderValue;

use crate::CacheableHttpRequest;

pub struct Path<E> {
    inner: E,
    resource: ResourceDef,
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
            resource: ResourceDef::try_from(resource).unwrap(),
        }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Path<E>
where
    ReqBody: Send + 'static,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let mut path = actix_router::Path::new(subject.parts().uri.path());
        self.resource.capture_match_info(&mut path);
        let mut matched_parts = path
            .iter()
            .map(|(key, value)| KeyPart {
                key: key.to_owned(),
                value: Some(value.to_owned()),
            })
            .collect::<Vec<_>>();
        let mut parts = self.inner.get(subject).await;
        parts.append(&mut matched_parts);
        parts
    }
}
