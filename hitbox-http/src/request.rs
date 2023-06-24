use std::any::type_name;

use async_trait::async_trait;
use futures::StreamExt;
use hitbox::{
    cache::{CacheKey, CacheableRequest},
    predicates::Predicate,
    Cacheable,
};
use hitbox_backend::CachePolicy;
use http::Request;
use hyper::Body;

pub struct CacheableHttpRequest {
    request: Request<Body>,
}

impl CacheableHttpRequest {
    pub fn from_request(request: Request<Body>) -> Self {
        Self { request }
    }

    pub fn into_request(self) -> Request<Body> {
        let uri = self.request.uri().clone();
        let body = self.request.into_body();
        let stream = body.map(|v| v);
        Request::builder()
            .uri(uri)
            .body(Body::wrap_stream(stream))
            .unwrap()
    }
}

#[async_trait]
impl CacheableRequest for CacheableHttpRequest {
    async fn cache_policy<P>(
        self,
        predicates: &[P],
    ) -> (CachePolicy<CacheKey>, CacheableHttpRequest)
    where
        P: Predicate<Self> + Send + Sync,
    {
        unimplemented!()
    }
}

#[async_trait]
impl Cacheable for CacheableHttpRequest {
    async fn cache_key(&self) -> Result<String, hitbox::CacheError> {
        let body = self.request.body();
        Ok(format!(
            "{}::{}",
            self.request.method(),
            self.request.uri().path()
        ))
    }

    fn cache_key_prefix(&self) -> String {
        format!("{}::", self.request.uri().scheme_str().unwrap_or("cache"))
    }
}

// impl<'a> From<&'a Request<hyper::Body>> for CacheableRequest<'a, hyper::Body> {
//     fn from(request: &'a Request<hyper::Body>) -> Self {
//         Self::from_request(request)
//     }
// }
