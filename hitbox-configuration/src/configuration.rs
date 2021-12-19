use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::backend::Backends;
use crate::cache::{Cache, OverriddenCache};
use crate::endpoint::Endpoint;
use crate::policy::Policy;
use crate::server::Server;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration<CacheType> {
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
    groups: HashMap<String, CacheType>,
    /// All used endpoint.
    endpoints: Vec<Endpoint<CacheType>>,
}

struct Wrapper<T>(pub T);

impl Wrapper<&HashMap<String, OverriddenCache>> {
    fn merge(&self, cache: &Cache) -> HashMap<String, Cache> {
        self.0
            .into_iter()
            .map(|(key, overridden)| (key.clone(), overridden.merge(cache)))
            .collect()
    }
}

impl Wrapper<&Vec<Endpoint<OverriddenCache>>> {
    fn merge(&self, cache: &Cache) -> Vec<Endpoint<Cache>> {
        self.0
            .into_iter()
            .map(|endpoint| endpoint.merge(cache))
            .collect()
    }
}

impl From<Configuration<OverriddenCache>> for Configuration<Cache> {
    fn from(overridden: Configuration<OverriddenCache>) -> Self {
        let groups = Wrapper(&overridden.groups).merge(&overridden.cache);
        let endpoints = Wrapper(&overridden.endpoints).merge(&overridden.cache);
        Self {
            server: overridden.server,
            upstreams: overridden.upstreams,
            backends: overridden.backends,
            policies: overridden.policies,
            cache: overridden.cache,
            groups,
            endpoints,
        }
    }
}

#[cfg(test)]
mod test {
    extern crate spectral;

    use std::env;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    use crate::configuration::Configuration;
    use crate::cache::{Cache, OverriddenCache};

    fn read_test_yaml() -> Configuration<OverriddenCache> {
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
        let merged: Configuration<Cache> = configuration.into();
        dbg!(merged);
    }
}
