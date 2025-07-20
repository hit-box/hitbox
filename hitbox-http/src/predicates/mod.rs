use std::{fmt::Debug, marker::PhantomData};

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

use crate::{CacheableHttpRequest, CacheableHttpResponse};

pub mod conditions;
pub mod request;
pub mod response;

pub struct NeutralRequestPredicate<ReqBody> {
    _req: PhantomData<fn(ReqBody) -> ReqBody>,
}

impl<ReqBody> Debug for NeutralRequestPredicate<ReqBody> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NeutralRequestPredicate")
            .field("_req", &self._req)
            .finish()
    }
}

impl<ReqBody> NeutralRequestPredicate<ReqBody> {
    pub fn new() -> Self {
        NeutralRequestPredicate { _req: PhantomData }
    }
}

#[async_trait]
impl<ReqBody> Predicate for NeutralRequestPredicate<ReqBody>
where
    ReqBody: Send + 'static,
{
    type Subject = CacheableHttpRequest<ReqBody>;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}

impl<ReqBody> Default for NeutralRequestPredicate<ReqBody> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct NeutralResponsePredicate<ResBody> {
    _res: PhantomData<fn(ResBody) -> ResBody>,
}

impl<ResBody> NeutralResponsePredicate<ResBody> {
    pub fn new() -> Self {
        NeutralResponsePredicate { _res: PhantomData }
    }
}

impl<ResBody> Debug for NeutralResponsePredicate<ResBody> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NeutralResponsePredicate")
            // .field("_res", &self._res)
            .finish()
    }
}

#[async_trait]
impl<ResBody> Predicate for NeutralResponsePredicate<ResBody>
where
    ResBody: Send + 'static,
{
    type Subject = CacheableHttpResponse<ResBody>;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}

impl<ResBody> Default for NeutralResponsePredicate<ResBody> {
    fn default() -> Self {
        Self::new()
    }
}
