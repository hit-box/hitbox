use std::marker::PhantomData;

use hitbox_core::CacheKey;

use crate::serializer::SerializerError;

pub trait KeySerializer {
    type Output;

    fn serialize(key: &CacheKey) -> Result<Self::Output, SerializerError>;
}

pub struct UrlEncodedSerializer<Output = String> {
    _output: PhantomData<Output>,
}

impl KeySerializer for UrlEncodedSerializer<String> {
    type Output = String;

    fn serialize(key: &CacheKey) -> Result<Self::Output, SerializerError> {
        let parts = key
            .parts()
            .map(|part| (part.key(), part.value()))
            .collect::<Vec<_>>();
        serde_urlencoded::to_string(&parts).map_err(|err| SerializerError::Serialize(Box::new(err)))
    }
}
