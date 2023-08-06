use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::{CacheBackend, CacheableResponse, CachedValue};
use hitbox_tarantool::TarantoolBackendBuilder;
use rusty_tarantool::tarantool::{ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use testcontainers::{clients, core::WaitFor, Image};

#[derive(Debug)]
pub struct Tarantool {
    env_vars: HashMap<String, String>,
}

impl Default for Tarantool {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("TARANTOOL_USER_NAME".to_owned(), "hitbox".to_owned());
        env_vars.insert("TARANTOOL_USER_PASSWORD".to_owned(), "hitbox".to_owned());

        Self { env_vars }
    }
}

impl Image for Tarantool {
    type Args = ();

    fn name(&self) -> String {
        "tarantool/tarantool".to_owned()
    }

    fn tag(&self) -> String {
        "latest".to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::Healthcheck]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![3301]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[tokio::test]
async fn test_start() {
    let docker = clients::Cli::default();
    let container = docker.run(Tarantool::default());
    let port = container
        .ports()
        .map_to_host_port_ipv4(3301)
        .unwrap()
        .to_string();
    let backend = TarantoolBackendBuilder::default()
        .port(port.clone())
        .build();
    backend.start().await.unwrap();
    let tarantool =
        ClientConfig::new(format!("{}:{}", "127.0.0.1", port), "hitbox", "hitbox").build();
    let space_exists: (bool,) = tarantool
        .eval(
            "return box.space[...] and true or false",
            &("hitbox_cache".to_string(),),
        )
        .await
        .unwrap()
        .decode()
        .unwrap();
    assert!(space_exists.0);
    let fiber_exists: (bool,) = tarantool
        .eval(
            "return _G[...] and true or false",
            &("__hitbox_cache_fiber".to_string(),),
        )
        .await
        .unwrap()
        .decode()
        .unwrap();
    assert!(fiber_exists.0);
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Test {
    a: i32,
    b: String,
}

#[async_trait]
impl CacheableResponse for Test {
    type Cached = Self;

    async fn into_cached(self) -> Self::Cached {
        self
    }
    async fn from_cached(cached: Self::Cached) -> Self {
        cached
    }
}

impl Test {
    pub fn new() -> Self {
        Self {
            a: 42,
            b: "nope".to_owned(),
        }
    }
}

#[tokio::test]
async fn test_set() {
    let docker = clients::Cli::default();
    let container = docker.run(Tarantool::default());
    let port = container
        .ports()
        .map_to_host_port_ipv4(3301)
        .unwrap()
        .to_string();
    let backend = TarantoolBackendBuilder::default()
        .port(port.clone())
        .build();
    backend.start().await.unwrap();

    let key = "test_key".to_owned();
    let value = CachedValue::new(Test::new(), Utc::now());
    backend.set::<Test>(key.clone(), value, None).await.unwrap();

    let tarantool =
        ClientConfig::new(format!("{}:{}", "127.0.0.1", port), "hitbox", "hitbox").build();

    tarantool
        .prepare_fn_call("box.space.hitbox_cache:get")
        .bind_ref(&("test_key",))
        .unwrap()
        .execute()
        .await
        .unwrap();
}
