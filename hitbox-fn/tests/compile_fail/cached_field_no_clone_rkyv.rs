use hitbox_derive::CacheableResponse;
use serde::{Deserialize, Serialize};

/// A type that does NOT implement Clone.
#[derive(Debug, Serialize, Deserialize)]
pub struct NoClone(String);

#[derive(Debug, Serialize, Deserialize, CacheableResponse)]
pub struct Response {
    pub no_clone: NoClone,
    #[cacheable_response(skip)]
    pub skipped: Option<String>,
}

fn main() {}
