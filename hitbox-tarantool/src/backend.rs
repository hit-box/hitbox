use async_trait::async_trait;
use hitbox_backend::{
    serializer::SerializableCachedValue, BackendError, BackendResult, CacheBackend, DeleteStatus,
};
use hitbox_core::{CacheKey, CacheableResponse, CachedValue};
use rusty_tarantool::tarantool::{Client, ClientConfig, ExecWithParamaters};
use serde::{Deserialize, Serialize};
use std::io::Error;
use typed_builder::TypedBuilder;

const TARANTOOL_INIT_LUA: &str = include_str!("init.lua");

/// Tarantool cache backend based on rusty_tarantool crate.
///
/// # Examples
/// ```
/// use hitbox_tarantool::Tarantool;
///
/// #[tokio::main]
/// async fn main() {
///     let mut backend = Tarantool::builder().build();
///     // backend.init().await.unwrap();
/// }
/// ```
#[derive(Clone, TypedBuilder)]
#[builder(build_method(vis="", name=__build))]
pub struct Tarantool {
    #[builder(default = "hitbox".to_string())]
    user: String,
    #[builder(default = "hitbox".to_string())]
    password: String,
    #[builder(default = "127.0.0.1".to_string())]
    host: String,
    #[builder(default = "3301".to_string())]
    port: String,
}

pub struct TarantoolBackend {
    client: Client,
}

#[allow(non_camel_case_types)]
impl<
        __user: ::typed_builder::Optional<String>,
        __password: ::typed_builder::Optional<String>,
        __host: ::typed_builder::Optional<String>,
        __port: ::typed_builder::Optional<String>,
    > TarantoolBuilder<(__user, __password, __host, __port)>
{
    pub fn build(self) -> TarantoolBackend {
        let t = self.__build();
        let client =
            ClientConfig::new(format!("{}:{}", t.host, t.port), t.user, t.password).build();
        TarantoolBackend { client }
    }
}

impl TarantoolBackend {
    /// Init backend and configure tarantool instance
    /// This function is idempotent
    pub async fn init(&mut self) -> BackendResult<()> {
        self.client
            .eval(TARANTOOL_INIT_LUA, &("hitbox_cache",))
            .await
            .map_err(|err| BackendError::InternalError(Box::new(err)))?;

        Ok(())
    }

    fn map_err(err: Error) -> BackendError {
        BackendError::InternalError(Box::new(err))
    }
}

#[doc(hidden)]
#[derive(Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub ttl: Option<u32>,
    pub value: SerializableCachedValue<T>,
}

#[async_trait]
impl CacheBackend for TarantoolBackend {
    async fn get<T>(&self, key: &CacheKey) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        self.client
            .prepare_fn_call("hitbox.get")
            .bind_ref(&(key.serialize()))
            .map_err(TarantoolBackend::map_err)?
            .execute()
            .await
            .map_err(TarantoolBackend::map_err)?
            .decode_single::<Option<CacheEntry<T::Cached>>>()
            .map_err(TarantoolBackend::map_err)
            .map(|v| v.map(|v| v.value.into_cached_value()))
    }

    async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let result: bool = self
            .client
            .prepare_fn_call("hitbox.delete")
            .bind_ref(&(key.serialize()))
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
        key: &CacheKey,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync,
    {
        let entry: CacheEntry<T::Cached> = CacheEntry {
            key: key.serialize(),
            ttl,
            value: value.clone().into(),
        };
        self.client
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
