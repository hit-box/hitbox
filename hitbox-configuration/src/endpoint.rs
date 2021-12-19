use serde::{Deserialize, Serialize};

use crate::cache::{OverriddenCache, Cache};
use crate::request::Request;
use crate::response::Response;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Endpoint<CacheType> {
    #[serde(flatten)]
    cache: CacheType,
    path: String,
    request: Option<Request>,
    response: Option<Response>,
}

impl Endpoint<OverriddenCache> {
    pub(crate) fn merge(&self, cache: &Cache) -> Endpoint<Cache> {
        Endpoint {
            cache: self.cache.merge(cache),
            path: self.path.clone(),
            request: self.request.clone(),
            response: self.response.clone(),
        }
    }
}