use std::marker::PhantomData;

use hitbox_backend::serializer::{JsonSerializer, Serializer};

use crate::backend::RedisBackend;
use crate::error::Error;

struct ClusterNode {}

pub struct Cluster<S = JsonSerializer<String>> {
    nodes: Vec<ClusterNode>,
    _ser: PhantomData<S>,
}

impl<S> Default for Cluster<S> {
    fn default() -> Self {
        Self {
            nodes: vec![],
            _ser: PhantomData,
        }
    }
}

impl<S> Cluster<S> {
    pub fn new() -> Self {
        Cluster::default()
    }

    pub fn build(self) -> Result<RedisBackend<S>, Error> {
        unimplemented!()
    }
}
