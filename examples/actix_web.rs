use actix::prelude::*;
use hitbox::{Cache, Cacheable};
use actix_derive::Message;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;

fn fibonacci(n: u8) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

struct FibonacciActor;

impl Actor for FibonacciActor {
    type Context = SyncContext<Self>;
}

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "u64")]
struct GetNumber {
    number: u8,
}

impl Handler<GetNumber> for FibonacciActor {
    type Result = <GetNumber as Message>::Result;

    fn handle(&mut self, msg: GetNumber, _ctx: &mut Self::Context) -> Self::Result {
        fibonacci(msg.number)
    }
}

async fn index(
    n: web::Path<u8>,
    fib: web::Data<Addr<FibonacciActor>>,
    cache: web::Data<Addr<Cache>>,
) -> impl Responder {
    let query = GetNumber {
        number: n.into_inner(),
    };
    let number = cache.send(query.into_cache(&fib)).await.unwrap().unwrap();
    HttpResponse::Ok().body(format!("Generate Fibonacci number {}", number))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let fib = { SyncArbiter::start(3, move || FibonacciActor) };
    let cache = Cache::new().await.unwrap().start();

    HttpServer::new(move || {
        App::new()
            .data(fib.clone())
            .data(cache.clone())
            .route("/fibonacci/{num}", web::get().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
