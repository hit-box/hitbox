use actix::prelude::*;
use hitbox_backend::{Delete, DeleteStatus, Get, Lock, LockStatus, Set};
use hitbox_redis::{error::Error, RedisBuilder};
use tokio::time::{sleep, Duration};

#[actix_rt::test]
async fn test_rw() -> Result<(), Error> {
    let addr = RedisBuilder::single_new().await?.finish().start();
    let message = Set {
        key: "key".to_owned(),
        value: b"value".to_vec(),
        ttl: None,
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");
    let res = addr
        .send(Get {
            key: message.key.clone(),
        })
        .await;
    assert_eq!(res.unwrap().unwrap(), Some(message.value));

    let res = addr.send(Delete { key: message.key }).await;
    res.unwrap().unwrap();
    Ok(())
}

#[actix_rt::test]
async fn test_set_expired() -> Result<(), Error> {
    let addr = RedisBuilder::single_new().await?.finish().start();
    let message = Set {
        key: "key_expired".to_owned(),
        value: b"value".to_vec(),
        ttl: Some(1),
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");

    let res = addr
        .send(Get {
            key: message.key.clone(),
        })
        .await;
    assert_eq!(res.unwrap().unwrap(), Some(message.value));

    sleep(Duration::from_secs(1)).await;

    let res = addr
        .send(Get {
            key: message.key.clone(),
        })
        .await;
    assert_eq!(res.unwrap().unwrap(), None);
    Ok(())
}

#[actix_rt::test]
async fn test_delete() -> Result<(), Error> {
    let addr = RedisBuilder::single_new().await?.finish().start();
    let message = Set {
        key: "another_key".to_owned(),
        value: b"value".to_vec(),
        ttl: Some(1),
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");

    let res = addr
        .send(Delete {
            key: message.key.clone(),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(res, DeleteStatus::Deleted(1));

    sleep(Duration::from_secs(1)).await;

    let res = addr
        .send(Delete {
            key: message.key.clone(),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(res, DeleteStatus::Missing);
    Ok(())
}

#[actix_rt::test]
async fn test_lock() -> Result<(), Error> {
    let addr = RedisBuilder::single_new().await?.finish().start();
    let message = Lock {
        key: "lock_key".to_owned(),
        ttl: 1,
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Acquired);

    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Locked);

    sleep(Duration::from_secs(1)).await;

    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Acquired);
    Ok(())
}
