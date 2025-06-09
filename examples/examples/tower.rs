use bytes::Bytes;
use hitbox_moka::MokaBackend;
// use hitbox_stretto::StrettoBackend;
use hitbox_tower::Cache;
use http_body_util::Full;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use http::{Request, Response};

async fn handle(_: Request<Full<Bytes>>) -> http::Result<Response<Full<Bytes>>> {
    Ok(Response::new("Hello, World!".into()))
    // Err(http::Error::from(Method::from_bytes(&[0x01]).unwrap_err()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let inmemory = MokaBackend::builder(10_000).build();
    // let inmemory = StrettoBackend::builder(10_000_000).finalize().unwrap();
    // let redis = RedisBackend::builder().build().unwrap();

    let service = tower::ServiceBuilder::new()
        .layer(Cache::builder().backend(inmemory).build())
        // .layer(Cache::builder().backend(redis).build())
        // .layer(
        //     Cache::builder()
        //         .backend(Arc::new(redis) as Arc<dyn Backend>)
        //         .build(),
        // )
        .service_fn(handle);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    // loop {
    //     let (stream, _) = listener.accept().await?;
    //     let io = TokioIo::new(stream);
    //
    //     tokio::task::spawn(async move {
    //         if let Err(err) = http1::Builder::new()
    //             .serve_connection(io, service_fn(echo))
    //             .await
    //         {
    //             println!("Error serving connection: {:?}", err);
    //         }
    //     });
    // }
    // Server::bind(&addr)
    //     .serve(Shared::new(service))
    //     .await
    //     .expect("server error");
    Ok(())
}
