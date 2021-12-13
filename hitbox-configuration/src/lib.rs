use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::backend::Backends;
use crate::cache::{Cache, OverriddenCache};
use crate::endpoint::Endpoint;
use crate::policy::Policy;
use crate::server::Server;

mod backend;
mod body;
mod cache;
mod endpoint;
mod field;
mod headers;
mod policy;
mod request;
mod response;
mod server;
mod status_code;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    /// Hitbox Server network settings.
    server: Server,
    /// All served applications with their names.
    upstreams: HashMap<String, Server>,
    /// All used stores.
    backends: Vec<Backends>,
    /// Predefined combinations of cache policies.
    policies: HashMap<String, Policy>,
    /// Common cache settings for the entire Application.
    cache: Cache,
    /// Predefined sets of backend, upstream & policy.
    groups: HashMap<String, OverriddenCache>,
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

    use crate::*;

    fn read_test_yaml() -> Configuration {
        let path = Path::new("src/test.yaml");
        let mut path_to_file = env::current_dir().unwrap();
        path_to_file.push(path);
        let mut test_yaml = File::open(&path).unwrap();
        let mut s = String::new();
        let _ = test_yaml.read_to_string(&mut s);
        let res = serde_yaml::from_str(s.as_str());
        res.unwrap()
    }

    #[test]
    fn test_base() {
        let configuration = read_test_yaml();
        dbg!(configuration);
    }
}
