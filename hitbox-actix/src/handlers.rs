//! Actix Handler<QueryCache> implementation. 

use crate::{ActixAdapter, CacheActor, QueryCache};
use actix::{
    dev::{MessageResponse, ResponseFuture, ToEnvelope},
    prelude::*,
};
use hitbox::states::initial::Initial;
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
            let initial_state = Initial::new(settings, adapter_result?);
            initial_state
                .transitions()
                .await
        })
    }
}
