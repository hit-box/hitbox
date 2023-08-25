use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hitbox_backend::{
    serializer::SerializableCachedValue, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use hitbox_tarantool::{backend::CacheEntry, backend::TarantoolBackend, Tarantool};
use once_cell::sync::Lazy;
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, thread, time::Duration};
use testcontainers::{clients, core::WaitFor, Container, Image};

static DOCKER: Lazy<clients::Cli> = Lazy::new(|| clients::Cli::default());

impl Image for TarantoolImage {
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

#[derive(Debug)]
struct TarantoolImage {
    env_vars: HashMap<String, String>,
}

impl<'a> TarantoolImage {
    async fn start() -> TarantoolContainer<'a> {
        let container = DOCKER.run(TarantoolImage::default());
        let port = &container.ports().map_to_host_port_ipv4(3301).unwrap();
        let client =
            ClientConfig::new(format!("{}:{}", "127.0.0.1", &port), "hitbox", "hitbox").build();
        let mut backend = Tarantool::builder().port(port.to_string()).build();
        backend.init().await.unwrap();
        TarantoolContainer {
            _container: container,
            client,
            backend,
        }
    }
}

struct TarantoolContainer<'a> {
    _container: Container<'a, TarantoolImage>,
    client: Client,
    backend: TarantoolBackend,
}

impl<'a> TarantoolContainer<'a> {
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
}

impl Default for TarantoolImage {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("TARANTOOL_USER_NAME".to_owned(), "hitbox".to_owned());
        env_vars.insert("TARANTOOL_USER_PASSWORD".to_owned(), "hitbox".to_owned());

        Self { env_vars }
    }
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

impl Default for Test {
    fn default() -> Self {
        Self {
            a: 42,
            b: "nope".to_owned(),
        }
    }
}

#[tokio::test]
async fn test_init() {
    let t = TarantoolImage::start().await;

    let space_exists: (bool,) = t
        .eval(
            "return box.space[...] and true or false",
            &("hitbox_cache",),
        )
        .await;
    assert!(space_exists.0);

    let fiber_exists: (bool,) = t
        .eval(
            "return _G[...] and true or false",
            &("__hitbox_cache_fiber",),
        )
        .await;
    assert!(fiber_exists.0);
}

#[tokio::test]
async fn test_set() {
    let t = TarantoolImage::start().await;
    let key = "test_key".to_string();
    let dt = "2012-12-12T12:12:12Z";
    let ttl = 42;
    let value = CachedValue::new(Test::default(), DateTime::from_str(&dt).unwrap());

    t.backend
        .set::<Test>(key.clone(), &value, Some(ttl))
        .await
        .unwrap();

    // let result: Vec<CacheEntry> = t.call("get", &(key.clone())).await;
    let result = t
        .client
        .prepare_fn_call(format!("box.space.hitbox_cache:get"))
        .bind_ref(&key)
        .unwrap()
        .execute()
        .await
        .unwrap()
        .decode_single::<CacheEntry<Test>>()
        .unwrap();

    assert_eq!(&result.ttl.unwrap(), &ttl);
    assert_eq!(&result.value.into_cached_value().data, &Test::default());
}

#[tokio::test]
async fn test_expire() {
    let t = TarantoolImage::start().await;
    let key = "test_key".to_owned();
    let dt = "2012-12-12T12:12:12Z";
    let value = CachedValue::new(Test::default(), DateTime::from_str(&dt).unwrap());

    t.backend
        .set::<Test>(key.clone(), &value, Some(0))
        .await
        .unwrap();

    thread::sleep(Duration::from_secs(1));

    let result = t
        .client
        .prepare_fn_call(format!("box.space.hitbox_cache:get"))
        .bind_ref(&key)
        .unwrap()
        .execute()
        .await
        .unwrap()
        .decode_result_set::<CacheEntry<Test>>()
        .unwrap();
    assert!(result.is_empty())
}

#[tokio::test]
async fn test_delete() {
    let t = TarantoolImage::start().await;
    let key = "test_key";
    let dt: DateTime<Utc> = DateTime::from_str(&"2012-12-12T12:12:12Z").unwrap();
    let value = Test::default();
    let cached_value = SerializableCachedValue::new(&value, dt);
    let entry = CacheEntry {
        key: key.into(),
        ttl: Some(42),
        value: cached_value,
    };

    let status = t.backend.delete(key.to_string()).await.unwrap();
    assert_eq!(status, DeleteStatus::Missing);

    t.client
        .prepare_fn_call(format!("box.space.hitbox_cache:replace"))
        .bind_ref(&entry)
        .unwrap()
        .execute()
        .await
        .unwrap();

    let status = t.backend.delete(key.to_string()).await.unwrap();
    assert_eq!(status, DeleteStatus::Deleted(1));

    let result = t
        .client
        .prepare_fn_call(format!("box.space.hitbox_cache:get"))
        .bind_ref(&key)
        .unwrap()
        .execute()
        .await
        .unwrap()
        .decode_result_set::<CacheEntry<Test>>()
        .unwrap();

    assert!(result.is_empty())
}

#[tokio::test]
async fn test_get() {
    let t = TarantoolImage::start().await;
    let key = "test_key";
    let dt: DateTime<Utc> = DateTime::from_str(&"2012-12-12T12:12:12Z").unwrap();

    let value = Test::default();
    let cached_value = SerializableCachedValue::new(&value, dt);
    let entry = CacheEntry {
        key: key.into(),
        ttl: Some(42),
        value: cached_value,
    };

    t.client
        .prepare_fn_call(format!("box.space.hitbox_cache:replace"))
        .bind_ref(&entry)
        .unwrap()
        .execute()
        .await
        .unwrap();

    let data = t.backend.get::<Test>(key.into()).await.unwrap().unwrap();

    assert_eq!(data.data, value);
    assert_eq!(data.expired, dt);
}
