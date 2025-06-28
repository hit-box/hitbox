use serde::{Deserialize, Serialize};

pub use predicates::request::Request;

pub mod predicates;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Endpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
}
