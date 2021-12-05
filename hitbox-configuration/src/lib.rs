use crate::backend::Backend;
use crate::cache::Cache;
use crate::endpoint::Endpoint;
use crate::group::Group;
use crate::policy::Policy;
use crate::server::Server;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod backend;
mod cache;
mod endpoint;
mod group;
mod policy;
mod server;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    server: Server,
    upstreams: HashMap<String, Server>,
    backends: HashMap<String, Backend>,
    policies: HashMap<String, Policy>,
    cache: Cache,
    groups: HashMap<String, Group>,
    endpoints: Vec<Endpoint>,
}

#[cfg(test)]
mod test {
    use crate::policy::{CacheStatus, Conf, InnerState, LockStatus, StaleStatus};
    use crate::server::Protocol;
    use crate::*;
    use std::env;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    fn read_test_yaml() -> Configuration {
        let path = Path::new("src/test.yaml");
        let mut path_to_file = env::current_dir().unwrap();
        path_to_file.push(path);
        let mut test_yaml = File::open(&path).unwrap();
        let mut s = String::new();
        test_yaml.read_to_string(&mut s);
        let res: Result<Configuration, _> = serde_yaml::from_str(s.as_str());
        res.unwrap()
    }

    #[test]
    fn test_base() {
        let configuration = read_test_yaml();
        dbg!(configuration);
        // let policy = Policy {
        //     cache: CacheStatus::Enabled(InnerState
        //     {
        //         lock: LockStatus::Local,
        //         stale: StaleStatus::Enabled }
        //     )
        // };
        // let mut conf = Conf { policies: HashMap::new() };
        // conf.policies.insert(String::from("main"), policy);
        // let res = serde_yaml::to_string(&conf).unwrap();
        // println!("{}", res);
    }
}
