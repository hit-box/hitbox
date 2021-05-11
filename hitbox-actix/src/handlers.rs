use crate::{ActixAdapter, CacheActor, QueryCache};
use actix::{
    dev::{MessageResponse, ResponseFuture, ToEnvelope},
    prelude::*,
};
use hitbox::settings::InitialCacheSettings;
use hitbox::states::initial::InitialState;
use hitbox::transition_groups::{only_cache, stale, upstream};
use hitbox::{
    dev::{Backend, Delete, Get, Lock, Set},
    CacheError, Cacheable, CacheableResponse,
};
use serde::{de::DeserializeOwned, Serialize};

impl<'a, A, M, B> Handler<QueryCache<A, M>> for CacheActor<B>
where
    B: Actor + Backend,
    <B as Actor>::Context:
        ToEnvelope<B, Get> + ToEnvelope<B, Set> + ToEnvelope<B, Lock> + ToEnvelope<B, Delete>,
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static,
    M::Result: MessageResponse<A, M> + CacheableResponse + std::fmt::Debug + Send,
    <<M as actix::Message>::Result as CacheableResponse>::Cached: Serialize + DeserializeOwned,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, CacheError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let adapter_result = ActixAdapter::new(msg, self.backend.clone()); // @TODO: remove clone
        let settings = self.settings.clone();
        Box::pin(async move {
            // let adapter = adapter_result?;
            let initial_state = InitialState {
                adapter: adapter_result?,
                settings,
            };
            match initial_state.settings {
                InitialCacheSettings::CacheDisabled => {
                    upstream::transition(initial_state).await.result()
                }
                InitialCacheSettings::CacheEnabled => {
                    only_cache::transition(initial_state).await.result()
                }
                InitialCacheSettings::CacheStale => stale::transition(initial_state).await.result(),
                InitialCacheSettings::CacheLock => unimplemented!(),
                InitialCacheSettings::CacheStaleLock => unimplemented!(),
            }
        })
    }
}
