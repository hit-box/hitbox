use std::marker::PhantomData;

use hitbox_backend::serializer::{JsonSerializer, Serializer};

use crate::backend::RedisBackend;
use crate::error::Error;

pub struct Sentinel<S = JsonSerializer<String>> {
    _ser: PhantomData<S>,
}

impl<S> Default for Sentinel<S> {
    fn default() -> Self {
        Self { _ser: PhantomData }
    }
}

impl<S> Sentinel<S> {
    pub fn new() -> Self {
        Sentinel::default()
    }

    pub fn build(self) -> Result<RedisBackend<S>, Error> {
        unimplemented!()
    }
}
