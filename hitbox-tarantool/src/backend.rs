use async_trait::async_trait;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};

#[derive(Clone)]
pub struct TarantoolBackend {
    client: Client,
}

impl TarantoolBackend {
    pub fn new() -> Result<TarantoolBackend, BackendError> {
        Ok(Self::builder().build())
    }

    /// Creates new TarantoolBackend builder with default settings.
    pub fn builder() -> TarantoolBackendBuilder {
        TarantoolBackendBuilder::default()
    }
}

/// Part of builder pattern implementation for TarantoolBackend actor.
pub struct TarantoolBackendBuilder {
    user: String,
    password: String,
    host: String,
    port: String,
}

impl Default for TarantoolBackendBuilder {
    fn default() -> Self {
        Self {
            user: "hitbox".to_owned(),
            password: "hitbox".to_owned(),
            host: "127.0.0.1".to_owned(),
            port: "3301".to_owned(),
        }
    }
}

impl TarantoolBackendBuilder {
    pub fn user(mut self, user: String) -> Self {
        self.user = user;
        self
    }

    pub fn password(mut self, password: String) -> Self {
        self.password = password;
        self
    }

    pub fn host(mut self, host: String) -> Self {
        self.host = host;
        self
    }

    pub fn port(mut self, port: String) -> Self {
        self.port = port;
        self
    }

    pub fn build(self) -> TarantoolBackend {
        let client = ClientConfig::new(
            format!("{}:{}", self.host, self.port),
            self.user,
            self.password,
        )
        .build();

        TarantoolBackend { client }
    }
}

#[async_trait]
impl CacheBackend for TarantoolBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let client = self.client.clone();

        // TODO
        let response = client
            .prepare_fn_call("test")
            .bind_ref(&("aa", "aa"))
            .unwrap()
            .bind(1)
            .unwrap()
            .execute()
            .await
            .unwrap();
        response
            .decode::<String>()
            .map(|value| {
                Some(
                    JsonSerializer::<Vec<u8>>::deserialize(value.into())
                        .map_err(BackendError::from)
                        .unwrap(),
                )
            })
            .map_err(|err| BackendError::InternalError(Box::new(err)))
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        todo!()
    }

    async fn set<T>(
        &self,
        key: String,
        value: CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        <T as CacheableResponse>::Cached: serde::Serialize + Send,
    {
        todo!()
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}
