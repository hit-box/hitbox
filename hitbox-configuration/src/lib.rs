use std::collections::HashMap;
use crate::server::Server;
use crate::upstream::Upstream;
use crate::backend::Backend;
use crate::policy::Policy;
use crate::cache::Cache;
use crate::group::Group;
use crate::endpoint::Endpoint;

mod policy;
mod server;
mod upstream;
mod backend;
mod cache;
mod group;
mod endpoint;

pub struct Configuration {
    pub server: Server,
    pub upstreams: HashMap<String, Upstream>,
    pub backends: HashMap<String, Backend>,
    pub policies: HashMap<String, Policy>,
    pub cache: Cache,
    pub groups: HashMap<String, Group>,
    pub endpoints: Vec<Endpoint>,
}
