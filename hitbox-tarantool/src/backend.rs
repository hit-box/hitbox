use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use derive_builder::Builder;
use hitbox_backend::{
    serializer::SerializableCachedValue, BackendError, BackendResult, CacheBackend,
    CacheableResponse, CachedValue, DeleteStatus,
};
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};

const TARANTOOL_INIT_LUA: &str = include_str!("init.lua");

/// Tarantool cache backend based on rusty_tarantool crate.
///
/// # Examples
/// ```
/// use hitbox_tarantool::TarantoolBackendBuilder;
///
/// #[tokio::main]
/// async fn main() {
///     let mut backend = TarantoolBackendBuilder::default()
///         .build()
///         .unwrap();
///     // backend.init().await.unwrap();
/// }
/// ```
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

    /// Init backend and configure tarantool instance
    /// This function is idempotent
    pub async fn init(&mut self) -> BackendResult<()> {
        if self.client.is_none() {
            let client = ClientConfig::new(
                format!("{}:{}", self.host, self.port),
                self.user.clone(),
                self.password.clone(),
            )
            .build();
            self.client = Some(client);
        }

        self.client()?
            .eval(TARANTOOL_INIT_LUA, &("hitbox_cache",))
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;

        Ok(())
    }

    fn map_err(err: Error) -> BackendError {
        BackendError::InternalError(Box::new(err))
    }
}

#[derive(Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub ttl: Option<u32>,
    pub value: SerializableCachedValue<T>,
}

#[async_trait]
impl CacheBackend for TarantoolBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        self.client()?
            .prepare_fn_call("hitbox.get")
            .bind_ref(&(key))
            .map_err(TarantoolBackend::map_err)?
            .execute()
            .await
            .map_err(TarantoolBackend::map_err)?
            .decode_single::<Option<CacheEntry<T::Cached>>>()
            .map_err(TarantoolBackend::map_err)
            .map(|v| v.map(|v| v.value.into_cached_value()))
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        let result: bool = self
            .client()?
            .prepare_fn_call("hitbox.delete")
            .bind_ref(&(key))
            .map_err(TarantoolBackend::map_err)?
            .execute()
            .await
            .map_err(TarantoolBackend::map_err)?
            .decode_single()
            .map_err(TarantoolBackend::map_err)?;
        match result {
            true => Ok(DeleteStatus::Deleted(1)),
            false => Ok(DeleteStatus::Missing),
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
        let entry: CacheEntry<T::Cached> = CacheEntry {
            key,
            ttl,
            value: value.clone().into(),
        };
        self.client()?
            .prepare_fn_call("hitbox.set")
            .bind_ref(&entry)
            .map_err(TarantoolBackend::map_err)?
            .execute()
            .await
            .map(|_| ())
            .map_err(TarantoolBackend::map_err)
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}
