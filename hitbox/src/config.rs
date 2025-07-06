use crate::policy::PolicyConfig;
use crate::predicate::Predicate;
use crate::Extractor;

pub type RequestPredicate<Req> =
    Box<dyn Predicate<Subject = Req> + Send + Sync>;

pub type ResponsePredicate<Res> =
    Box<dyn Predicate<Subject = Res> + Send + Sync>;

pub type RequestExtractor<Req> =
    Box<dyn Extractor<Subject = Req> + Send + Sync>;

pub trait CacheConfig<Req, Res> {
    type RequestBody;
    type ResponseBody;

    fn request_predicates(&self) -> RequestPredicate<Self::RequestBody>;

    fn response_predicates(&self) -> ResponsePredicate<Self::ResponseBody>;

    fn extractors(&self) -> RequestExtractor<Self::RequestBody>;

    fn policy(&self) -> PolicyConfig;
}
