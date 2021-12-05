use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::backend::Backend;
use crate::cache::Cache;
use crate::endpoint::Endpoint;
use crate::policy::Policy;
use crate::server::Server;

mod backend;
mod cache;
mod endpoint;
mod policy;
mod server;
mod headers;
mod query;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    /// Hitbox Server network settings.
    server: Server,
    /// All served applications with their names.
    upstreams: HashMap<String, Server>,
    /// All used stores.
    backends: HashMap<String, Backend>,
    /// Predefined combinations of cache policies.
    policies: HashMap<String, Policy>,
    /// Common cache settings for the entire Application.
    cache: Cache,
    /// Predefined sets of backend, upstream & policy.
    groups: HashMap<String, Cache>,
    /// All used endpoint.
    endpoints: Vec<Endpoint>,
}

#[cfg(test)]
mod test {
    extern crate spectral;

    use std::env;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    use spectral::prelude::*;

    use crate::*;
    use crate::policy::{CacheStatus, Conf, InnerState, LockStatus, StaleStatus};
    use crate::server::Protocol;

    fn read_test_yaml() -> Configuration {
        let path = Path::new("src/test.yaml");
        let mut path_to_file = env::current_dir().unwrap();
        path_to_file.push(path);
        let mut test_yaml = File::open(&path).unwrap();
        let mut s = String::new();
        test_yaml.read_to_string(&mut s);
        serde_yaml::from_str(s.as_str()).unwrap()
    }

    #[test]
    fn test_base() {
        let configuration = read_test_yaml();
        dbg!(configuration);
    }
}
