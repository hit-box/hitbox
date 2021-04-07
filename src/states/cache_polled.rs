use crate::states::{FinishState, UpstreamPolled};
use crate::{Database, Error, Message, Sender, CacheError};
use actix::Message;

#[derive(Debug)]
pub enum CacheStatus<R> {
    Hit(R),
    Miss,
}

#[derive(Debug)]
pub struct CachePolled<M, R>
where
    M: Message<Result = R>,
{
    pub cache_status: CacheStatus<R>,
    pub message: M,
}

impl<M, R> CachePolled<M, R>
where
    M: Message<Result = R>,
{
    pub fn poll_upstream(self) -> UpstreamPolled<R> {
        println!("-> Poll upstream");
        let result = Database.send(&self.message);
        UpstreamPolled {
            upstream_result: result,
        }
    }

    pub fn finish(self) -> Result<FinishState<R>, CacheError> {
        println!("-> Finish");
        match self.cache_status {
            CacheStatus::Hit(cached_value) => Ok(FinishState {
                result: cached_value,
            }),
            CacheStatus::Miss => Err(Error::InvalidTransition(
                "Transition from CachePolled to Finish are prohibited for CacheStatus::Miss"
                    .to_owned(),
            )),
        }
    }
}
