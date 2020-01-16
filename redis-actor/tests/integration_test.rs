use actix_rt;
use actix::prelude::*;
use redis_actor::actor::{Redis, Get, Set, Delete, DeleteStatus, Lock, LockStatus};
use tokio::time::{delay_for, Duration};

#[actix_rt::test]
async fn test_rw() {
    let addr = Redis::new().await.start();
    let message = Set {
        key: "key".to_owned(), 
        value: "value".to_owned(), 
        ttl: None 
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");
    let res = addr.send(Get { 
        key: message.key.clone()
    }).await;
    assert_eq!(res.unwrap().unwrap(), Some(message.value));

    let res = addr.send(Delete { 
        key: message.key 
    }).await;
    res.unwrap().unwrap();
}

#[actix_rt::test]
async fn test_set_expired() {
    let addr = Redis::new().await.start();
    let message = Set {
        key: "key".to_owned(), 
        value: "value".to_owned(), 
        ttl: Some(1)
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");

    let res = addr.send(Get { 
        key: message.key.clone()
    }).await;
    assert_eq!(res.unwrap().unwrap(), Some(message.value));

    delay_for(Duration::from_secs(1)).await;

    let res = addr.send(Get { 
        key: message.key.clone()
    }).await;
    assert_eq!(res.unwrap().unwrap(), None);
}

#[actix_rt::test]
async fn test_delete() {
    let addr = Redis::new().await.start();
    let message = Set {
        key: "another_key".to_owned(), 
        value: "value".to_owned(), 
        ttl: Some(1)
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, "OK");

    let res = addr.send(Delete { 
        key: message.key.clone()
    }).await.unwrap().unwrap();
    assert_eq!(res, DeleteStatus::Deleted(1));

    delay_for(Duration::from_secs(1)).await;

    let res = addr.send(Delete { 
        key: message.key.clone()
    }).await.unwrap().unwrap();
    assert_eq!(res, DeleteStatus::Missing);
}

#[actix_rt::test]
async fn test_lock() {
    let addr = Redis::new().await.start();
    let message = Lock {
        key: "lock_key".to_owned(), 
        ttl: 1,
    };
    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Acquired);

    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Locked);

    delay_for(Duration::from_secs(1)).await;

    let res = addr.send(message.clone()).await.unwrap().unwrap();
    assert_eq!(res, LockStatus::Acquired);
}
