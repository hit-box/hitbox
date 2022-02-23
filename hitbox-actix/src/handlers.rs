//! Actix Handler<QueryCache> implementation.

use crate::{ActixAdapter, CacheActor, QueryCache};
use actix::{
    dev::{MessageResponse, ResponseFuture, ToEnvelope},
    prelude::*,
};
use hitbox::states::initial::Initial;
use hitbox::{
    dev::{Backend, Delete, Get, Lock, Set},
    CacheError, Cacheable,
};
use hitbox_backend::{CacheBackend, CacheableResponse};
use serde::{de::DeserializeOwned, Serialize};

impl<'a, A, M, B> Handler<QueryCache<A, M>> for CacheActor<B>
where
    B: CacheBackend + Unpin + 'static + Send + Sync,
    A: Actor + Handler<M> + Send,
    M: Message + Cacheable + Send + 'static + Sync,
    M::Result: MessageResponse<A, M> + CacheableResponse + std::fmt::Debug + Send + Sync,
    <<M as actix::Message>::Result as CacheableResponse>::Cached: Serialize + DeserializeOwned,
    <A as Actor>::Context: ToEnvelope<A, M>,
{
    type Result = ResponseFuture<Result<<M as Message>::Result, CacheError>>;

    fn handle(&mut self, msg: QueryCache<A, M>, _: &mut Self::Context) -> Self::Result {
        let adapter_result = ActixAdapter::new(msg, self.backend.clone());
        let settings = self.settings.clone();
        Box::pin(async move {
            let initial_state = Initial::new(settings, adapter_result?);
            initial_state.transitions().await
        })
    }
}
