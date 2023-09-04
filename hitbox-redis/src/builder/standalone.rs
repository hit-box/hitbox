use std::marker::PhantomData;

use fred::clients::RedisClient;
use fred::types::{Blocking, RedisConfig, RespVersion, ServerConfig};
use hitbox_backend::serializer::{JsonSerializer, Serializer};

use crate::backend::RedisBackend;
use crate::error::Error;

pub struct Standalone<S = JsonSerializer<String>> {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    database: Option<u8>,
    _ser: PhantomData<S>,
}

impl<S> Default for Standalone<S> {
    fn default() -> Self {
        Standalone {
            username: None,
            password: None,
            host: Some("127.0.0.1".to_owned()),
            port: Some(6379),
            database: None,
            _ser: PhantomData,
        }
    }
}

impl<S: Serializer> Standalone<S> {
    pub fn from_url(connection_url: &str) -> Result<Standalone<S>, Error> {
        let cfg = RedisConfig::from_url(connection_url)?;
        if cfg.server.is_centralized() {
            let server = cfg.server.hosts()[0];
            Ok(Self {
                username: cfg.username,
                password: cfg.password,
                database: cfg.database,
                host: Some(server.host.to_string()),
                port: Some(server.port),
                _ser: PhantomData,
            })
        } else {
            todo!()
        }
    }

    pub fn set_username(self, value: String) -> Self {
        Standalone {
            username: Some(value),
            ..self
        }
    }

    pub fn set_password(self, value: String) -> Self {
        Standalone {
            password: Some(value),
            ..self
        }
    }

    pub fn set_host(self, value: String) -> Self {
        Standalone {
            host: Some(value),
            ..self
        }
    }

    pub fn set_port(self, value: u16) -> Self {
        Standalone {
            port: Some(value),
            ..self
        }
    }

    pub fn set_database(self, value: u8) -> Self {
        Standalone {
            database: Some(value),
            ..self
        }
    }

    pub fn with_serializer<S2: Serializer>(self) -> Standalone<S2> {
        Standalone {
            username: self.username,
            password: self.password,
            host: self.host,
            port: self.port,
            database: self.database,
            _ser: PhantomData::<S2>,
        }
    }

    pub fn build(self) -> Result<RedisBackend<S>, Error> {
        let host = self
            .host
            .ok_or_else(|| Error::Builder("Please setup host".to_owned()))?;
        let port = self
            .port
            .ok_or_else(|| Error::Builder("Please setup port".to_owned()))?;
        let config = RedisConfig {
            fail_fast: true,
            server: ServerConfig::new_centralized(host, port),
            blocking: Blocking::Block,
            username: self.username,
            password: self.password,
            version: RespVersion::RESP2,
            database: self.database,
        };

        let client = RedisClient::new(config, None, None);
        Ok(RedisBackend {
            client,
            _ser: self._ser,
        })
    }
}
