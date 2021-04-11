use actix::{Actor, Handler, Message};
use crate::{Cacheable, QueryCache};
use actix::dev::{MessageResponse, ToEnvelope};
use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::adapted::AdapterResult;

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

impl<A, M, T> RuntimeAdapter for ActixAdapter<A, M>
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