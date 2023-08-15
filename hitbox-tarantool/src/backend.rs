use std::io::{Error, ErrorKind};

use async_trait::async_trait;
use derive_builder::Builder;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};

const TARANTOOL_INIT_LUA: &str = include_str!("init.lua");

/// Tarantool cache backend based on rusty_tarantool crate.
///
/// # Examples
/// ```
/// use hitbox_tarantool::TarantoolBackend;
///
/// #[tokio::main]
/// async fn main() {
///     let mut backend = TarantoolBackendBuilder::default()
///         .build()
///         .unwrap();
///     backend.init().await.unwrap();
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
