use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use derive_builder::Builder;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};

#[derive(Clone, Builder)]
pub struct TarantoolBackend {
    #[builder(default = "\"hitbox\".to_string()")]
    user: String,
    #[builder(default = "\"hitbox\".to_string()")]
    password: String,
    #[builder(default = "\"127.0.0.1\".to_string()")]
    host: String,
    #[builder(default = "\"3301\".to_string()")]
    port: String,
    #[builder(setter(skip))]
    client: Option<Client>,
}

impl TarantoolBackend {
    fn client(&self) -> BackendResult<Client> {
        let err = Error::new(ErrorKind::Other, "Backend is not initialized");
        self.client
            .clone()
            .ok_or(BackendError::InternalError(Box::new(err)))
    }

    pub async fn init(&mut self) -> BackendResult<()> {
        let client = ClientConfig::new(
            format!("{}:{}", self.host, self.port),
            self.user.clone(),
            self.password.clone(),
        )
        .build();
        self.client = Some(client);

        self.client()?
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

    fn map_err(err: Error) -> BackendError {
        BackendError::InternalError(Box::new(err))
    }

    async fn call<T>(&self, cmd: &str, params: &T) -> BackendResult<Vec<CacheEntry>>
    where
        T: Serialize,
    {
        let result: Vec<CacheEntry> = self
            .client()?
            .prepare_fn_call(format!("box.space.hitbox_cache:{}", cmd))
            .bind_ref(params)
            .map_err(TarantoolBackend::map_err)?
            .execute()
            .await
            .map_err(TarantoolBackend::map_err)?
            .decode_result_set()
            .map_err(TarantoolBackend::map_err)?;

        Ok(result)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CacheEntry {
    pub key: String,
    pub ttl: Option<u32>,
    pub value: String,
}

#[async_trait]
impl CacheBackend for TarantoolBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let entries = self.call("get", &(key)).await?;
        let entry = match entries.first() {
            Some(v) => Some(
                JsonSerializer::<String>::deserialize(v.value.clone())
                    .map_err(BackendError::from)?,
            ),
            None => None,
        };

        Ok(entry)
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        let entries = self.call("delete", &(key)).await?;
        if entries.is_empty() {
            Ok(DeleteStatus::Missing)
        } else {
            Ok(DeleteStatus::Deleted(1))
        }
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
        let serialized_value =
            JsonSerializer::<String>::serialize(value).map_err(BackendError::from)?;
        self.call("replace", &(key, ttl, serialized_value)).await?;
        Ok(())
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}
