use crate::settings::{InitialCacheSettings, CacheSettings, Status};
use std::fmt::Debug;
use actix::{Message, Actor, Handler};
use crate::{CacheError, Cacheable, QueryCache};
use crate::dev::BackendError;
use std::pin::Pin;
use std::future::Future;
use actix::dev::{MessageResponse, ToEnvelope};
use std::marker::PhantomData;
use crate::adapted::runtime_adapter::RuntimeAdapter;

pub enum CacheStatus {
    Miss,
    Hit,
}

#[derive(Debug)]
pub struct InitialState<A>
where
    A: RuntimeAdapter,
{
    // pub settings: InitialStateSettings,
    pub adapter: A,
}

pub struct UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
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
    A: RuntimeAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledError),
}

impl<A> InitialState<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
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
