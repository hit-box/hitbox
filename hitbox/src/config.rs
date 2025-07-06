use std::sync::Arc;

use crate::policy::PolicyConfig;
use crate::predicate::Predicate;
use crate::Extractor;

pub type RequestPredicate<Req> = Box<dyn Predicate<Subject = Req> + Send + Sync>;

pub type ResponsePredicate<Res> = Box<dyn Predicate<Subject = Res> + Send + Sync>;

pub type RequestExtractor<Req> = Box<dyn Extractor<Subject = Req> + Send + Sync>;

pub trait CacheConfig<Req, Res> {
    type RequestBody;
    type ResponseBody;

    fn request_predicates(&self) -> impl Predicate<Subject = Req> + Send + Sync + 'static;

    fn response_predicates(&self) -> impl Predicate<Subject = Res> + Send + Sync + 'static;

    fn extractors(&self) -> impl Extractor<Subject = Req> + Send + Sync + 'static;

    fn policy(&self) -> PolicyConfig;
}

impl<T, Req, Res> CacheConfig<Req, Res> for Arc<T>
where
    T: CacheConfig<Req, Res>,
{
    type RequestBody = T::RequestBody;
    type ResponseBody = T::ResponseBody;

    fn request_predicates(&self) -> impl Predicate<Subject = Req> + Send + Sync + 'static {
        self.as_ref().request_predicates()
    }

    fn response_predicates(&self) -> impl Predicate<Subject = Res> + Send + Sync + 'static {
        self.as_ref().response_predicates()
    }

    fn extractors(&self) -> impl Extractor<Subject = Req> + Send + Sync + 'static {
        self.as_ref().extractors()
    }

    fn policy(&self) -> PolicyConfig {
        self.as_ref().policy()
    }
}
