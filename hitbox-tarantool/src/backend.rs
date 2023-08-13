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

impl TarantoolBackend {
    pub async fn init(&self) -> BackendResult<()> {
        let client = self.client.clone();
        client
            .eval(
                "
                    local space_name = ...
                    box.schema.space.create(space_name, { if_not_exists = true })
                    box.space[space_name]:create_index('primary', {
                        parts = { { 1, 'string' } },
                        if_not_exists = true,
                    })

                    if not _G.__hitbox_cache_fiber then
                        _G.__hitbox_cache_fiber = require('fiber').create(function()
                            local fiber = require('fiber')
                            fiber.name('hitbox_cache_fiber')
                            while true do
                                local ok, res = pcall(function()
                                    for _, t in box.space[space_name]:pairs() do
                                        if t[2] <= fiber.time() then
                                            box.space[space_name]:delete(t[1])
                                        end
                                    end
                                end)

                                if not ok then
                                    require('log').error(err)
                                end

                                fiber.testcancel()
                                fiber.sleep(1)
                            end
                        end)
                    end
                ",
                &("hitbox_cache".to_string(),),
            )
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;
        Ok(())
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
        let response = client
            .prepare_fn_call("box.space.hitbox_cache:get")
            .bind_ref(&(key))
            .map_err(|err| BackendError::InternalError(Box::new(err)))?
            .execute()
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;
        response
            .decode_result_set::<(String, Option<u32>, String)>()
            .map(|value| {
                Some(
                    JsonSerializer::<String>::deserialize(value.first().unwrap().2.clone())
                        .map_err(BackendError::from)
                        .unwrap(),
                )
            })
            .map_err(|err| BackendError::InternalError(Box::new(err)))
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        let client = self.client.clone();
        let response = client
            .prepare_fn_call("box.space.hitbox_cache:delete")
            .bind_ref(&(key))
            .map_err(|err| BackendError::InternalError(Box::new(err)))?
            .execute()
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;
        let result = response
            .decode_result_set::<(String, Option<u32>, String)>()
            .map(|_| Some(DeleteStatus::Deleted(1)))
            .map_err(|err| BackendError::InternalError(Box::new(err)))?
            .unwrap_or(DeleteStatus::Missing);
        Ok(result)
    }

    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync,
    {
        let client = self.client.clone();
        let serialized_value =
            JsonSerializer::<String>::serialize(value).map_err(BackendError::from)?;
        client
            .prepare_fn_call("box.space.hitbox_cache:replace")
            .bind_ref(&(key, ttl, serialized_value))
            .map_err(|err| BackendError::InternalError(Box::new(err)))?
            .execute()
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;
        Ok(())
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}
