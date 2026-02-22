use hitbox_derive::CacheableResponse;
use serde::{Deserialize, Serialize};

/// A type that does NOT implement Default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoDefault(String);

#[derive(Debug, Clone, Serialize, Deserialize, CacheableResponse)]
pub struct Response {
    pub id: u64,
    #[cacheable_response(skip)]
    pub no_default: NoDefault,
}

fn main() {}
