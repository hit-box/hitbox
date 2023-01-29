use std::{
    fmt::Debug,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{FutureExt, ready};
use hitbox::{
    dev::{BackendError, CacheBackend},
    settings::{CacheSettings, Status},
    states::initial::Initial,
    CacheError, CachePolicy, Cacheable, CacheableResponse, CachedValue,
};
use hitbox_redis::RedisBackend;
use hitbox_tokio::FutureAdapter;
use http::Request;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tower::{Layer, Service};
use pin_project_lite::pin_project;

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

struct Adapter<'a, S, Req>
where
    S: Service<Request<Req>>,
{
    upstream: &'a mut S,
    req: Request<Req>,
}

impl<'a, S, Req> Adapter<'a, S, Req>
where
    S: Service<Request<Req>>,
{
    // fn new(upstream: &mut S, req: &Request<Req>) -> Self {
    // Adapter { upstream.call(), req }
    // }

    async fn call(mut self) -> Result<S::Response, S::Error> {
        self.upstream.call(self.req).await
    }
}

impl<S, Req, B, Res> Service<Request<Req>> for CacheService<S, B>
where
    S: Service<Request<Req>, Response = Response<Res>>,
    Request<Req>: Debug,
    B: CacheBackend,
{
    type Response = S::Response;
    type Error = S::Error;
    // type Future = ResponseFut<S, Res, Req, S::Error, B>;
    type Future = ResponseFut<S, Res, Req, S::Error, Pin<Box<dyn Future<Output = Result<Option<CachedValue<Respon<Res>>>, BackendError>>>>>;
    // type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Req>) -> Self::Future {
        // let adapter = Adapter::new(&mut self.upstream, req);
        let cache_query = req.cache_query();
        let poll_cache = self.backend.get(cache_query.cache_key);
        ResponseFut {
            state: CacheState::CachePoll { poll_cache },
            // upstream: &mut self.upstream,
            // backend: self.backend.clone(),
        }

        /*ResponseFuture {
            // fut: self.upstream.call(req).map(Box::new(|res| {
            // match res {
            // Ok(res) => Ok(Respon(res)),
            // Err(err) => Err(err),
            // }})),
            // tran: None,
            upstream: Box::pin(self.upstream.call(req)),
            poll_cache: None,
            cache_query,
            backend: backend,
            // fut: None,
            _res: PhantomData::default(),
            _e: PhantomData::default(),
        }*/
    }
}

use http::Response;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

#[derive(Debug, Clone)]
struct CacheQuery {
    cache_key: String,
}

impl Cacheable for CacheQuery {
    fn cache_key(&self) -> Result<String, hitbox::CacheError> {
        Ok(self.cache_key.clone())
    }

    fn cache_key_prefix(&self) -> String {
        "v1".to_string()
    }
}

trait Cacheable2 {
    fn cache_query(&self) -> CacheQuery;
}

impl<Req> Cacheable2 for Request<Req> {
    fn cache_query(&self) -> CacheQuery {
        CacheQuery {
            cache_key: self.method().to_string(),
        }
    }
}

struct Respon<T>(Response<T>);

impl<T> Debug for Respon<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ETO DEBUG")
    }
}

impl<T> Serialize for Respon<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("krevedko")
    }
}

impl<'de, T> Deserialize<'de> for Respon<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Ok(Respon(Response::new("hui")))
        unimplemented!()
    }
}

impl<T> CacheableResponse for Respon<T> {
    type Cached = String;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        CachePolicy::NonCacheable(())
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::NonCacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        unimplemented!()
    }
}

pin_project! {
    pub struct ResponseFut<S, Res, Req, E, PollCache> 
    where
        S: Service<Request<Req>, Response = Response<Res>, Error = E>,
    {
        #[pin]
        state: CacheState<PollCache, S::Future>,
        // #[pin]
        // upstream: S,
        // backend: Arc<B>,
    }
}

pin_project! {
    #[project = CacheStateProj]
    enum CacheState<PollCache, PollUpstream> {
        CachePoll {
            #[pin]
            poll_cache: PollCache,
        },
        CahePolled {
            #[pin]
            poll_upstream: PollUpstream,
        }
    }
}

impl<S, Res, Req, E, PollCache> Future for ResponseFut<S, Res, Req, E, PollCache> 
where
    S: Service<Request<Req>, Response = Response<Res>, Error = E>,
    PollCache: Future<Output = Result<Option<CachedValue<Respon<Res>>>, BackendError>>,
{
    type Output = Result<Response<Res>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project(); 

        loop {
            match this.state.as_mut().project() {
                CacheStateProj::CachePoll { poll_cache } => {
                    let cache_polled = ready!(poll_cache.poll(cx));
                    match cache_polled {
                        Ok(Some(cached_value)) => {
                            let Respon(res) = cached_value.data;
                            return Poll::Ready(Ok(res))
                        },
                        _ => return Poll::Pending,
                    }
                }
                _ => unimplemented!(),
            }
        }
    }
}

pin_project! {
    pub struct ResponseFuture<S, Res, Req, E, B>
    where
        S: Service<Request<Req>>,
        // F: Future<Output=Result<S::Response, S::Error>>,
    {
        // #[pin]
        // fut: Option<Pin<Box<dyn Future<Output=Result<S::Response, S::Error>>>>>,
        // fut: Option<Box<dyn Future<Output=Result<S::Response, S::Error>>>>,

        // correct
        // #[pin]
        // fut: futures::future::Map<S::Future, Box<dyn FnOnce(Result<Response<Res>, S::Error>) -> Result<Respon<Res>, S::Error> + Send>>,
        // #[pin]
        // tran: Option<Pin<Box<dyn Future<Output = Result<Result<Respon<Res>, S::Error>, CacheError>>>>>,


        upstream: Pin<Box<dyn Future<Output = Result<Response<Res>, S::Error>> + Send>>,
        poll_cache: Option<Pin<Box<dyn Future<Output = Result<Option<CachedValue<Respon<Res>>>, BackendError>> + Send>>>,
        cache_query: CacheQuery,
        backend: Arc<B>,
        _res: PhantomData<Res>,
        _e: PhantomData<E>,
    }
}

impl<S, Res, Req, E, B> Future for ResponseFuture<S, Res, Req, E, B>
where
    S: Service<Request<Req>, Response = Response<Res>, Error = E>,
    B: CacheBackend,
    // F: Future<Output=Result<S::Response, S::Error>>,
    // F: Future<Output = Result<Response<Res>, E>>,
{
    type Output = Result<Response<Res>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        dbg!(&this.cache_query);
        // if this.poll_cache.as_mut().is_none() {
            // this.poll_cache.project().insert(Box::pin(
                // this.backend.get(this.cache_query.cache_key.clone()),
            // ));
        // }

        println!("++++++++++++++++++++++++++");
        println!("self.poll_cache is Some = {}", this.poll_cache.is_some());

        let cache_polled = this
            .poll_cache
            .as_mut()
            .map(|mut fut| fut.as_mut().poll(cx))
            .unwrap_or(Poll::Pending);
        dbg!(&cache_polled);

        Poll::Pending

        // let adapter = FutureAdapter::new(
        // move |_| this.fut,
        // this.cache_query.clone(),
        // this.backend.as_ref(),
        // )
        // .unwrap();
        // let settings = CacheSettings {
        // cache: Status::Enabled,
        // lock: Status::Disabled,
        // stale: Status::Disabled,
        // };

        // let initial_state = Initial::new(settings, adapter);
        // if this.tran.as_ref().is_none() {
        // this.tran.as_ref().insert(Box::pin(initial_state.transitions()));
        // // this.tran = Some();
        // }
        // match this.tran.get_unchecked_mut() {
        // Some(f) => match Pin::new(f).poll(cx) {
        // Poll::Ready(Ok(Ok(Respon(r)))) => Poll::Ready(Ok(r)),
        // _ => Poll::Pending,
        // }
        // None => Poll::Pending,
        // }

        // match Pin::new(&mut initial_state.transitions()).poll(cx) {
        // Poll::Ready(Ok(Ok(Respon(res)))) => Poll::Ready(Ok(res)),
        // _ => Poll::Pending,
        // }

        // this.fut.poll(cx)

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
        // let response = futures::ready!(Box::pin(&mut this.call(req)).poll(cx))?;
        // this.fut = Some(Box::pin(this.adapter.call()));
        // let response = futures::ready!(this.adapter.call().poll())?;
        // Poll::Ready(Ok(response))
        // Poll::Ready(Err(()))
        // res
        // Poll::Pending
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
