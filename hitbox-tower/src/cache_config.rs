use std::fmt::Debug;

use hitbox::Extractor;
use hitbox::policy::PolicyConfig;
use hitbox::predicate::Predicate;
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};

type RequestPredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>;

type ResponsePredicate<ResBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync>;

type RequestExtractor<ResBody> =
    Box<dyn Extractor<Subject = CacheableHttpRequest<ResBody>> + Send + Sync>;

pub trait CacheConfig {
    fn request_predicates<ReqBody>(&self) -> RequestPredicate<ReqBody>
    where
        ReqBody: Send + 'static;

    fn response_predicates<ResBody>(&self) -> ResponsePredicate<ResBody>
    where
        ResBody: Send + 'static;

    fn extractors<ReqBody>(&self) -> RequestExtractor<ReqBody>
    where
        ReqBody: Send + 'static + Debug;

    fn policy(&self) -> PolicyConfig;
}
