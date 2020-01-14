use actix::prelude::*;
use log::info;

pub struct Cache {}

impl Actor for Cache {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {
        info!("Cache actor started");
    }
}
