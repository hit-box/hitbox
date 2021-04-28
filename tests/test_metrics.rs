#[cfg(feature = "metrics")]
mod tests {
    use actix::prelude::*;
    use hitbox::metrics::CACHE_MISS_COUNTER;
    use hitbox::{dev::backend::MockBackend, CacheActor, CacheError, Cacheable};

    pub struct Upstream;

    impl Actor for Upstream {
        type Context = Context<Self>;
    }

    #[derive(Message)]
    #[rtype(result = "Result<i32, ()>")]
    pub struct Ping(i32);

    impl Cacheable for Ping {
        fn cache_message_key(&self) -> Result<String, CacheError> {
            Ok(format!("{}::{}", self.cache_key_prefix(), self.0))
        }
        fn cache_key_prefix(&self) -> String {
            "Ping".to_owned()
        }
    }

    impl Handler<Ping> for Upstream {
        type Result = Result<i32, ()>;

        fn handle(&mut self, msg: Ping, _: &mut Self::Context) -> Self::Result {
            if msg.0 > 0 {
                Ok(msg.0)
            } else {
                Err(())
            }
        }
    }

    #[derive(Message)]
    #[rtype(result = "i32")]
    pub struct Pong;

    impl Cacheable for Pong {
        fn cache_message_key(&self) -> Result<String, CacheError> {
            Ok(self.cache_key_prefix())
        }
        fn cache_key_prefix(&self) -> String {
            "Pong".to_owned()
        }
    }

    impl Handler<Pong> for Upstream {
        type Result = i32;

        fn handle(&mut self, _msg: Pong, _: &mut Self::Context) -> Self::Result {
            42
        }
    }

    #[actix_rt::test]
    async fn test_miss_counter_metric() {
        let backend = MockBackend::new().start();
        let cache = CacheActor::builder().build(backend).start();
        let upstream = Upstream {}.start();
        let res = cache.send(Ping(8).into_cache(&upstream)).await.unwrap();
        assert_eq!(res.unwrap(), Ok(8));
        let res = cache.send(Ping(-42).into_cache(&upstream)).await.unwrap();
        assert_eq!(res.unwrap(), Err(()));
        let res = cache.send(Pong.into_cache(&upstream)).await.unwrap();
        assert_eq!(res.unwrap(), 42);

        assert_eq!(
            2,
            CACHE_MISS_COUNTER
                .with_label_values(&["Ping", "Upstream"])
                .get()
        );
        assert_eq!(
            1,
            CACHE_MISS_COUNTER
                .with_label_values(&["Pong", "Upstream"])
                .get()
        );
    }
}
