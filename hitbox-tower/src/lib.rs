pub mod service;
// pub mod future;
pub mod state;

/*use std::{
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::RwLock,
    task::{Context, Poll},
};

use futures::FutureExt;
use hitbox::{
    dev::CacheBackend,
    settings::{CacheSettings, Status},
    states::initial::Initial,
    CachePolicy, Cacheable, CacheableResponse,
};
use hitbox_redis::RedisBackend;
use hitbox_tokio::FutureAdapter;
use http::{Request, Response, StatusCode};
use serde::Serialize;
use tower::{filter::future::AsyncResponseFuture, Layer, Service};*/


/*pub struct CacheableRequest<T>(Request<T>);

impl<T> CacheableRequest<T> {
    pub fn into_inner(self) -> Request<T> {
        self.0
    }
}

impl<T> Cacheable for CacheableRequest<T> {
    fn cache_key(&self) -> Result<String, hitbox::CacheError> {
        Ok("hui".to_string())
    }

    fn cache_key_prefix(&self) -> String {
        "pizda::".to_string()
    }
}

pub struct CachedResponse<T>(T);

impl<T> Debug for CachedResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CachedResponse")
    }
}

impl<T> Serialize for CachedResponse<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("hui")
    }
}

impl<T> CacheableResponse for CachedResponse<T>
where
    T: HttpResponse,
{
    type Cached = String;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        CachePolicy::NonCacheable(())
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::NonCacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        CachedResponse(HttpResponse::from_string(cached))
    }
}

trait HttpResponse {
    fn into_response(&self) -> &str;
    fn from_string(str: String) -> Self;
}

impl<E> HttpResponse for Result<Response<String>, E> {
    fn into_response(&self) -> &str {
        "hui"
    }

    fn from_string(str: String) -> Self {
        Ok(Response::builder()
            .status(StatusCode::IM_A_TEAPOT)
            .body(str.into())
            .unwrap())
    }
}

#[derive(Clone, Debug)]
pub struct CacheService<S: Debug, B, U, F> {
    inner: S,
    backend: B,
    _p: PhantomData<U>,
    _f: PhantomData<F>
}

// fn call<'a, S, R>(
// inner: &'a mut S,
// ) -> impl FnMut(R) -> Pin<Box<dyn Future<Output = CachedResponse<S::Response>> + 'a>>
// where
// S: Service<R> + Clone,
// R: Clone + 'a,
// {
// |req: R| {
// let req2 = req.clone();
// Box::pin(async move {
// match inner.clone().call(req2.clone()).await {
// Ok(res) => CachedResponse(res),
// Err(_) => unimplemented!(),
// }
// })
// }
// }
//
impl<S, B, U, F> CacheService<S, B, U, F>
where
    S: Debug,
{
    async fn call_hui<R>(
        &mut self,
        CacheableRequest(req): CacheableRequest<R>,
    ) -> CachedResponse<S::Response>
    where
        S: Service<Request<R>>,
    {
        let res = match self.inner.call(req).await {
            Ok(res) => res,
            Err(_) => unimplemented!(),
        };
        CachedResponse(res)
    }
}

// impl<S, Request> Service<Request> for CacheService<S>
impl<S, T, B, U, F> Service<Request<T>> for CacheService<S, B, U, F>
where
    S: Service<Request<T>> + Debug + Sync + Send + Clone + 'static,
    T: Debug + Send + Sync + 'static,
    B: CacheBackend + Sync + Send,
    U: Send + Sync + From<String>,
    <S as Service<Request<T>>>::Response: HttpResponse + Send + Sync,
    <S as Service<Request<T>>>::Future: Send + Sync,
    <S as Service<Request<T>>>::Error: Send + Sync,
    Result<<S as Service<Request<T>>>::Response, <S as Service<Request<T>>>::Error>: HttpResponse + Clone,
    F: Future<Output=Result<S::Response, S::Error>> + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    // type Future = S::Future;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        dbg!(&req);
        let cr = CacheableRequest(req);
        dbg!(&cr.cache_key());

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let adapter = FutureAdapter::new(
            move |CacheableRequest(cr)| {
                inner.call(cr).map(CachedResponse)
            },
            cr,
            &self.backend,
        )
        .unwrap();

        let settings = CacheSettings {
            cache: Status::Enabled,
            lock: Status::Disabled,
            stale: Status::Disabled,
        };

        let initial_state = Initial::new(settings, adapter);
        Box::pin(async move {
            let CachedResponse(res) = match initial_state.transitions().await {
                Ok(res) => res,
                Err(_) => unimplemented!(),
            };
            res
        })
        // let a = initial_state.transitions()
            // .then(|res| async {
                // let CachedResponse(res) = res.unwrap();
                // res
            // });
        // Box::pin(a)
    }
}

pub struct Cache<B, U, F> {
    backend: B,
    _p: PhantomData<U>,
    _f: PhantomData<F>,
}

impl<S, B, U, F> Layer<S> for Cache<B, U, F>
where
    S: Debug,
    B: Clone,
{
    type Service = CacheService<S, B, U, F>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            backend: self.backend.clone(),
            _p: PhantomData::default(),
            _f: PhantomData::default(),
        }
    }
}

impl<B, U, F> Cache<B, U, F>
where
    B: CacheBackend,
{
    pub fn builder() -> CacheBuilder<RedisBackend, U, F> {
        CacheBuilder::default()
    }
}

pub struct CacheBuilder<B, U, F>
where
    B: CacheBackend,
{
    backend: Option<B>,
    _p: PhantomData<U>,
    _f: PhantomData<F>,
}

impl<B, U, F> CacheBuilder<B, U, F>
where
    B: CacheBackend,
{
    pub fn build(self) -> Cache<B, U, F> {
        Cache {
            backend: self.backend.unwrap(),
            _p: PhantomData::default(),
            _f: PhantomData::default(),
        }
    }
}

impl<U, F> Default for CacheBuilder<RedisBackend, U, F> {
    fn default() -> Self {
        Self {
            backend: Some(RedisBackend::new().unwrap()),
            _p: Default::default(),
            _f: Default::default(),
        }
    }
}*/
