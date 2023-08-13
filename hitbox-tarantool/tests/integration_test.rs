use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hitbox_backend::{CacheBackend, CacheableResponse, CachedValue, DeleteStatus};
use hitbox_tarantool::TarantoolBackendBuilder;
use rusty_tarantool::tarantool::{ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use testcontainers::{clients, core::WaitFor, Image};

#[derive(Debug)]
struct Tarantool {
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
async fn test_init() {
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
    backend.init().await.unwrap();
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct TarantoolTuple {
    key: String,
    ttl: Option<u32>,
    value: String,
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
    backend.init().await.unwrap();

    let key = "test_key".to_owned();
    let dt = "2012-12-12T12:12:12Z".to_string();
    let ttl = 42;
    let value = CachedValue::new(Test::new(), DateTime::from_str(&dt).unwrap());
    backend
        .set::<Test>(key.clone(), &value, Some(ttl))
        .await
        .unwrap();

    let tarantool =
        ClientConfig::new(format!("{}:{}", "127.0.0.1", port), "hitbox", "hitbox").build();

    let response = tarantool
        .prepare_fn_call("box.space.hitbox_cache:get")
        .bind_ref(&("test_key"))
        .unwrap()
        .execute()
        .await
        .unwrap()
        .decode_result_set::<TarantoolTuple>()
        .unwrap();

    assert_eq!(
        response.first().unwrap(),
        &TarantoolTuple {
            key,
            ttl: Some(ttl),
            value: r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string()
        }
    );
}

#[tokio::test]
async fn test_expire() {}

#[tokio::test]
async fn test_delete() {
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
    backend.init().await.unwrap();

    let key = "test_key".to_owned();
    let value = r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string();
    let ttl = 42;

    let tarantool =
        ClientConfig::new(format!("{}:{}", "127.0.0.1", port), "hitbox", "hitbox").build();

    tarantool
        .prepare_fn_call("box.space.hitbox_cache:replace")
        .bind_ref(&(key.clone(), ttl, value))
        .unwrap()
        .execute()
        .await
        .unwrap();

    let status = backend.delete(key.clone()).await.unwrap();

    assert_eq!(status, DeleteStatus::Deleted(1));

    let response = tarantool
        .prepare_fn_call("box.space.hitbox_cache:get")
        .bind_ref(&("test_key"))
        .unwrap()
        .execute()
        .await
        .unwrap()
        .decode_result_set::<TarantoolTuple>()
        .unwrap();

    assert_eq!(response.len(), 0)
}

#[tokio::test]
async fn test_get() {
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
    backend.init().await.unwrap();

    let key = "test_key".to_owned();
    let value = r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string();
    let ttl = 42;

    let tarantool =
        ClientConfig::new(format!("{}:{}", "127.0.0.1", port), "hitbox", "hitbox").build();

    tarantool
        .prepare_fn_call("box.space.hitbox_cache:replace")
        .bind_ref(&(key.clone(), ttl, value))
        .unwrap()
        .execute()
        .await
        .unwrap();

    let data = backend.get::<Test>(key.clone()).await.unwrap().unwrap();
    let dt: DateTime<Utc> = DateTime::from_str(&"2012-12-12T12:12:12Z").unwrap();

    assert_eq!(data.data, Test::new());
    assert_eq!(data.expired, dt);
}
