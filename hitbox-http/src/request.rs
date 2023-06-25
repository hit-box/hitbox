use std::any::type_name;

use async_trait::async_trait;
use futures::StreamExt;
use hitbox::{
    cache::{CacheKey, CacheableRequest},
    predicates::Predicate,
    Cacheable,
};
use hitbox_backend::CachePolicy;
use http::{request::Parts, Request};
use hyper::Body;

pub struct CacheableHttpRequest {
    parts: Parts,
    body: Body,
}

impl CacheableHttpRequest {
    pub fn from_request(request: Request<Body>) -> Self {
        let (parts, body) = request.into_parts();
        Self { parts, body }
    }

    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    //pub fn into_request(self) -> Request<Body> {
    //let uri = self.request.uri().clone();
    //let body = self.request.into_body();
    //let stream = body.map(|v| v);
    //Request::builder()
    //.uri(uri)
    //.body(Body::wrap_stream(stream))
    //.unwrap()
    //}
}

#[async_trait]
impl CacheableRequest for CacheableHttpRequest {
    async fn cache_policy<P>(self, predicates: &[P]) -> hitbox::cache::CachePolicy<Self>
    where
        P: Predicate<Self> + Send + Sync,
    {
        unimplemented!()
    }
}
