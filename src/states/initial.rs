use crate::settings::{InitialCacheSettings, CacheSettings, Status};
use std::fmt::Debug;
use actix::{Message, Actor, Handler};
use crate::{CacheError, Cacheable, QueryCache};
use crate::dev::BackendError;
use std::pin::Pin;
use std::future::Future;
use actix::dev::{MessageResponse, ToEnvelope};
use std::marker::PhantomData;

pub enum CacheStatus {
    Miss,
    Hit,
}

type AdapterResult<T> = Pin<Box<dyn Future<Output=Result<T, CacheError>>>>;

pub trait BackendAdapter
{
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
}

pub struct ActixAdapter<A, M>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
{
    message: QueryCache<A, M>,
}

impl<A, M> ActixAdapter<A, M>
where
    A: Actor + Handler<M>,
    M: Message + Cacheable + Send,
    M::Result: MessageResponse<A, M> + Send,
{
    pub fn new(message: QueryCache<A, M>) -> Self {
        Self { message }
    }
}

impl<A, M, T> BackendAdapter for ActixAdapter<A, M>
where
    A: Actor + Handler<M>,
    A::Context: ToEnvelope<A, M>,
    M: Message<Result = T> + Cacheable + Send + Clone + 'static,
    M::Result: MessageResponse<A, M> + Send,
{
    type UpstreamResult = T;

    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        let message = self.message.message.clone();
        let upstream = self.message.upstream.clone();
        Box::pin(async move {
            Ok(upstream.send(message).await.unwrap())
        })
    }
}

#[derive(Debug)]
pub struct InitialState<A>
where
    A: BackendAdapter,
{
    // pub settings: InitialStateSettings,
    pub adapter: A,
}

pub struct UpstreamPolledSuccessful<A, T>
where
    A: BackendAdapter,
{
    pub adapter: A,
    pub result: T
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: BackendAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: self.result }
    }
}

pub struct UpstreamPolledError {
    pub error: CacheError
}

impl UpstreamPolledError {
    pub fn finish(self) -> Finish<CacheError> {
        Finish { result: self.error }
    }
}

pub enum UpstreamPolled<A, T>
where
    A: BackendAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledError),
}

impl<A> InitialState<A>
where
    A: BackendAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: BackendAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
}

#[derive(Debug)]
pub struct Finish<T: Debug>
{
    result: T
}

impl<T> Finish<T>
where
    T: Debug
{
    pub fn result(self) -> T {
        self.result
    }
}
