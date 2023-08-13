use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hitbox_backend::{CacheBackend, CacheableResponse, CachedValue, DeleteStatus};
use hitbox_tarantool::TarantoolBackendBuilder;
use once_cell::sync::Lazy;
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use testcontainers::{clients, core::WaitFor, Container, Image};

static DOCKER: Lazy<clients::Cli> = Lazy::new(|| clients::Cli::default());

#[derive(Debug)]
struct Tarantool {
    env_vars: HashMap<String, String>,
}

impl<'a> Tarantool {
    fn start() -> StartedTarantool<'a> {
        let container = DOCKER.run(Tarantool::default());
        let port = &container.ports().map_to_host_port_ipv4(3301).unwrap();
        let client =
            ClientConfig::new(format!("{}:{}", "127.0.0.1", &port), "hitbox", "hitbox").build();
        StartedTarantool {
            _container: container,
            client,
            port: port.to_string(),
        }
    }
}

struct StartedTarantool<'a> {
    _container: Container<'a, Tarantool>,
    client: Client,
    port: String,
}

impl<'a> StartedTarantool<'a> {
    async fn eval<T, R>(&self, cmd: &str, params: &T) -> R
    where
        T: Serialize,
        R: Deserialize<'a>,
    {
        self.client
            .eval(cmd, params)
            .await
            .unwrap()
            .decode()
            .unwrap()
    }
    async fn call<T, R>(&self, cmd: &str, params: &T) -> Vec<R>
    where
        T: Serialize,
        R: Deserialize<'a>,
    {
        self.client
            .prepare_fn_call(cmd)
            .bind_ref(params)
            .unwrap()
            .execute()
            .await
            .unwrap()
            .decode_result_set()
            .unwrap()
    }
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
    let c = Tarantool::start();
    let backend = TarantoolBackendBuilder::default()
        .port(c.port.clone())
        .build();
    backend.init().await.unwrap();
    let space_exists: (bool,) = c
        .eval(
            "return box.space[...] and true or false",
            &("hitbox_cache".to_string(),),
        )
        .await;
    assert!(space_exists.0);
    let fiber_exists: (bool,) = c
        .eval(
            "return _G[...] and true or false",
            &("__hitbox_cache_fiber".to_string(),),
        )
        .await;
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
    let c = Tarantool::start();
    let backend = TarantoolBackendBuilder::default()
        .port(c.port.clone())
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

    let result: Vec<TarantoolTuple> = c.call("box.space.hitbox_cache:get", &(key.clone())).await;

    assert_eq!(
        result.first().unwrap(),
        &TarantoolTuple {
            key,
            ttl: Some(ttl),
            value: r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string()
        }
    );
}

#[tokio::test]
async fn test_delete() {
    let c = Tarantool::start();
    let backend = TarantoolBackendBuilder::default()
        .port(c.port.clone())
        .build();
    backend.init().await.unwrap();

    let key = "test_key".to_owned();
    let value = r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string();
    let ttl = 42;

    c.call::<_, (String, Option<u32>, String)>(
        "box.space.hitbox_cache:replace",
        &("test_key".to_string(), ttl, value),
    )
    .await;

    let status = backend.delete(key.clone()).await.unwrap();

    assert_eq!(status, DeleteStatus::Deleted(1));

    let result: Vec<(String, Option<u32>, String)> = c
        .call("box.space.hitbox_cache:get", &("test_key".to_string()))
        .await;

    assert_eq!(result.len(), 0)
}

#[tokio::test]
async fn test_get() {
    let c = Tarantool::start();
    let backend = TarantoolBackendBuilder::default()
        .port(c.port.clone())
        .build();
    backend.init().await.unwrap();

    let key = "test_key".to_owned();
    let value = r#"{"data":{"a":42,"b":"nope"},"expired":"2012-12-12T12:12:12Z"}"#.to_string();

    c.call::<_, (String, Option<u32>, String)>(
        "box.space.hitbox_cache:replace",
        &(key.clone(), 42, value)
    )
    .await;

    let data = backend.get::<Test>(key).await.unwrap().unwrap();
    let dt: DateTime<Utc> = DateTime::from_str(&"2012-12-12T12:12:12Z").unwrap();

    assert_eq!(data.data, Test::new());
    assert_eq!(data.expired, dt);
}
