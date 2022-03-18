use hitbox_backend::{Delete, DeleteStatus, Get, Lock, LockStatus, Set, CacheBackend, CacheableResponse, CachePolicy, CachedValue};
use hitbox_redis::{error::Error, RedisBackend};
use tokio::time::{sleep, Duration};
use serde::{Serialize, Deserialize};
use chrono::Utc;
use test_log::test;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Test {
    a: i32,
    b: String,
}

impl CacheableResponse for Test {
    type Cached = Self;

    fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
        CachePolicy::Cacheable(self)
    }

    fn from_cached(cached: Self::Cached) -> Self {
        cached
    }

    fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::Cacheable(self)
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

#[test(tokio::test)]
async fn test_rw() -> Result<(), Error> {
    tokio::time::pause();
    let backend = RedisBackend::new().unwrap();
    backend.start().await.unwrap();
    let key = "test_key".to_owned();
    let inner = Test::new();
    let value = CachedValue::new(inner.clone(), Utc::now());
    let res = backend.set(key.clone(), &value, None).await.unwrap();
    assert_eq!(res, ());
    let res = backend.get::<Test>(key.clone()).await.unwrap();
    assert_eq!(res.unwrap().into_inner(), CachedValue::new(inner, Utc::now()).into_inner());
    let res = backend.delete(key.clone()).await.unwrap();
    assert_eq!(res, DeleteStatus::Deleted(1));
    Ok(())
}

// #[actix_rt::test]
// async fn test_set_expired() -> Result<(), Error> {
    // let addr = RedisBackend::new().await?.start();
    // let message = Set {
        // key: "key_expired".to_owned(),
        // value: b"value".to_vec(),
        // ttl: Some(1),
    // };
    // let res = addr.send(message.clone()).await.unwrap().unwrap();
    // assert_eq!(res, "OK");

    // let res = addr
        // .send(Get {
            // key: message.key.clone(),
        // })
        // .await;
    // assert_eq!(res.unwrap().unwrap(), Some(message.value));

    // sleep(Duration::from_secs(1)).await;

    // let res = addr
        // .send(Get {
            // key: message.key.clone(),
        // })
        // .await;
    // assert_eq!(res.unwrap().unwrap(), None);
    // Ok(())
// }

// #[actix_rt::test]
// async fn test_delete() -> Result<(), Error> {
    // let addr = RedisBackend::new().await?.start();
    // let message = Set {
        // key: "another_key".to_owned(),
        // value: b"value".to_vec(),
        // ttl: Some(1),
    // };
    // let res = addr.send(message.clone()).await.unwrap().unwrap();
    // assert_eq!(res, "OK");

    // let res = addr
        // .send(Delete {
            // key: message.key.clone(),
        // })
        // .await
        // .unwrap()
        // .unwrap();
    // assert_eq!(res, DeleteStatus::Deleted(1));

    // sleep(Duration::from_secs(1)).await;

    // let res = addr
        // .send(Delete {
            // key: message.key.clone(),
        // })
        // .await
        // .unwrap()
        // .unwrap();
    // assert_eq!(res, DeleteStatus::Missing);
    // Ok(())
// }

// #[actix_rt::test]
// async fn test_lock() -> Result<(), Error> {
    // let addr = RedisBackend::new().await?.start();
    // let message = Lock {
        // key: "lock_key".to_owned(),
        // ttl: 1,
    // };
    // let res = addr.send(message.clone()).await.unwrap().unwrap();
    // assert_eq!(res, LockStatus::Acquired);

    // let res = addr.send(message.clone()).await.unwrap().unwrap();
    // assert_eq!(res, LockStatus::Locked);

    // sleep(Duration::from_secs(1)).await;

    // let res = addr.send(message.clone()).await.unwrap().unwrap();
    // assert_eq!(res, LockStatus::Acquired);
    // Ok(())
// }
