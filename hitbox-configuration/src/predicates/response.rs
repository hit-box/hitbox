use hitbox_http::CacheableHttpResponse;
use hitbox_http::predicates::NeutralResponsePredicate;
use serde::{Deserialize, Serialize};

type CorePredicate<ReqBody> =
    Box<dyn hitbox_core::Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(untagged)]
pub enum Response {
    #[default]
    Flat,
}

impl Response {
    pub fn into_predicates<Req>(&self) -> CorePredicate<Req>
    where
        Req: Send + 'static,
    {
        Box::new(NeutralResponsePredicate::new())
    }
}
