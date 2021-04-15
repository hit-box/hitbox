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
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledSuccessful, UpstreamPolledError};
use crate::states::cache_polled::{CachePolled, CachePolledSuccessful};
use crate::adapted::actix_runtime_adapter::CacheState;

#[derive(Debug)]
pub struct InitialState<A>
where
    A: RuntimeAdapter,
{
    pub settings: InitialCacheSettings,
    pub adapter: A,
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

    pub async fn poll_cache<T>(self) -> CachePolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_cache().await {
            Ok(CacheState::Actual(result)) => CachePolled::Successful(
                CachePolledSuccessful { adapter: self.adapter, result: result.into_inner() }
            ),
            _ => unreachable!()
            // Err(error) => CachePolled::Error(CachePolledError { error }),
        }
    }
}
