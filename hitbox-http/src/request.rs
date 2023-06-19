use std::any::type_name;

use async_trait::async_trait;
use futures::StreamExt;
use hitbox::Cacheable;
use http::Request;

pub struct CacheableRequest<Body> {
    request: Request<Body>,
}

impl CacheableRequest<hyper::Body> {
    pub fn from_request(request: Request<hyper::Body>) -> Self {
        Self { request }
    }

    pub fn into_origin(self) -> Request<hyper::Body> {
        let uri = self.request.uri().clone();
        let body = self.request.into_body();
        let stream = body.map(|v| v);
        Request::builder()
            .uri(uri)
            .body(hyper::Body::wrap_stream(stream))
            .unwrap()
    }
}

#[async_trait]
impl Cacheable for CacheableRequest<hyper::Body> {
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
