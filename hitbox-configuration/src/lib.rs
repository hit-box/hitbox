pub mod request;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Endpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: request::Request,
}
