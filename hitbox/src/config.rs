use std::sync::Arc;

use crate::Extractor;
use crate::policy::PolicyConfig;
use crate::predicate::Predicate;

pub type BoxPredicate<R> = Box<dyn Predicate<Subject = R> + Send + Sync>;
pub type BoxExtractor<Req> = Box<dyn Extractor<Subject = Req> + Send + Sync>;

pub trait CacheConfig<Req, Res> {
    fn request_predicates(&self) -> impl Predicate<Subject = Req> + Send + Sync + 'static;

    fn response_predicates(&self) -> impl Predicate<Subject = Res> + Send + Sync + 'static;

    fn extractors(&self) -> impl Extractor<Subject = Req> + Send + Sync + 'static;

    fn policy(&self) -> &PolicyConfig;
}

impl<T, Req, Res> CacheConfig<Req, Res> for Arc<T>
where
    T: CacheConfig<Req, Res>,
{
    fn request_predicates(&self) -> impl Predicate<Subject = Req> + 'static {
        self.as_ref().request_predicates()
    }

    fn response_predicates(&self) -> impl Predicate<Subject = Res> + 'static {
        self.as_ref().response_predicates()
    }

    fn extractors(&self) -> impl Extractor<Subject = Req> + 'static {
        self.as_ref().extractors()
    }

    fn policy(&self) -> &PolicyConfig {
        self.as_ref().policy()
    }
}
