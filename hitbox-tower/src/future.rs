use std::{
    fmt::Debug,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use http::Request;
use tower::{Layer, Service};

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> Clone for CacheService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S, Req, B, Res> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Response = Response<Res>> + Send,
    Request<Req>: Debug,
    B: CacheBackend,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S, Res, Req, S::Error>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Req>) -> Self::Future {
        ResponseFuture {
            inner: self.upstream,
            req,
            _res: PhantomData::default(),
            _e: PhantomData::default(),
        }
    }
}

use http::Response;
use pin_project_lite::pin_project;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

pin_project! {
    pub struct ResponseFuture<F, Res, Req, E> {
        //#[pin]
        inner: F,
        req: Request<Req>,
        _res: PhantomData<Res>,
        _e: PhantomData<E>,
    }
}

impl<F, Res, Req, E> Future for ResponseFuture<F, Res, Req, E>
where
    F: Service<Request<Req>, Response = Response<Res>, Error = E> + Send,
    //F: Future<Output = Result<Response<Res>, E>>,
{
    type Output = Result<Response<Res>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        //let this = self.project();
        let this = self.inner;
        let req = self.req;

        //let adapter = FutureAdapter::new(
        //    move |cr)| {
        //        inner.call(cr).map(CachedResponse)
        //    },
        //    cr,
        //    &self.backend,
        //)
        //.unwrap();

        //let a = futures::ready!(Pin::new(&mut this.backend.get::<String>("kek".to_owned())).poll(cx))
        //    .unwrap()
        //    .unwrap()
        //    .into_inner();
        //dbg!(a);
        let response = futures::ready!(Pin::new(&mut this.call(req)).poll(cx))?;
        Poll::Ready(Ok(response))
    }
}

pub struct Cache<B> {
    backend: Arc<B>,
}

impl<B> Clone for Cache<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService {
            upstream,
            backend: self.backend.clone(),
        }
    }
}

impl<B> Cache<B>
where
    B: CacheBackend,
{
    pub fn builder() -> CacheBuilder<RedisBackend> {
        CacheBuilder::default()
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn backend(mut self, backend: B) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.unwrap()),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self { backend: None }
    }
}
