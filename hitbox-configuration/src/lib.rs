use serde::{Deserialize, Serialize};

pub use predicates::request::Request;

use crate::extractors::Extractor;

pub mod extractors;
pub mod predicates;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Endpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub extractors: Vec<Extractor>,
}
