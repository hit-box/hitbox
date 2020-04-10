use redis::RedisError;

#[derive(Debug)]
pub enum Error {
    Redis(RedisError),
}

impl From<RedisError> for Error {
    fn from(error: RedisError) -> Self {
        Error::Redis(error)
    }
}
