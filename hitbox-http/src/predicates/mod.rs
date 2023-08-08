use std::{
    marker::PhantomData,
    sync::{atomic::AtomicPtr, Arc, RwLock},
};

use async_trait::async_trait;
use hitbox_backend::predicates::{Predicate, PredicateResult};

use crate::{CacheableHttpRequest, CacheableHttpResponse};

pub mod body;
pub mod header;
pub mod path;
pub mod query;

pub struct NeutralPredicate<ReqBody> {
    _req: PhantomData<AtomicPtr<Box<ReqBody>>>, // FIX: NOT HEHE
}

impl<ReqBody> NeutralPredicate<ReqBody> {
    pub fn new() -> Self {
        NeutralPredicate { _req: PhantomData }
    }
}

#[async_trait]
impl<ReqBody> Predicate for NeutralPredicate<ReqBody>
where
    ReqBody: Send + 'static,
{
    type Subject = CacheableHttpRequest<ReqBody>;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}

pub struct NeutralResponsePredicate<ResBody> {
    _res: PhantomData<fn(ResBody) -> ResBody>, // FIX: HEHE
}

impl<ResBody> NeutralResponsePredicate<ResBody> {
    pub fn new() -> Self {
        NeutralResponsePredicate { _res: PhantomData }
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
