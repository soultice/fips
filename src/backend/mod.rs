// spawns the hyper server on a separate thread
use hyper::{Request, server::conn::http1};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use std::error::Error as StdError;
use tokio::task::JoinHandle;
use tokio::net::TcpListener;
use tokio::sync::Mutex as AsyncMutex;

use super::fips;
use super::PaintLogsCallbacks;
use super::utility::log::Loggable;
use crate::configuration::configuration::Config;

pub fn spawn_backend(
    configuration: Arc<AsyncMutex<Config>>,
    addr: SocketAddr,
    logger: Arc<PaintLogsCallbacks>,
) -> JoinHandle<Result<(), Box<dyn StdError + Send + Sync>>> {
    tokio::spawn(async move {
        let listener = TcpListener::bind(&addr).await?;
        log::info!("Listening on http://{}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let inner_configuration = configuration.clone();
            let inner_logger = logger.clone();

            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let innermost_configuration = inner_configuration.clone();
                let callbacks = Arc::new(AsyncMutex::new({
                    let logger = inner_logger.clone();
                    let func: Box<dyn Fn(Loggable) + Send + Sync> = Box::new(move |loggable: Loggable| {
                        (logger.0)(&loggable)
                    });
                    vec![func]
                }));
                let external_functions = Arc::new(crate::plugin_registry::ExternalFunctions::default());

                async move {
                    fips::handle_request(
                        req,
                        innermost_configuration,
                        callbacks,
                        &external_functions,
                    ).await
                }
            });

            let conn = http1::Builder::new()
                .serve_connection(io, service);

            // Spawn the connection to be driven in the background
            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    })
}

#[cfg(not(feature = "ui"))]
fn define_log_callbacks() -> PaintLogsCallbacks {
    use log::info;
    let log = Box::new(|message: &Loggable| info!("{:?}", message.message));
    PaintLogsCallbacks(log)
}

#[cfg(not(feature = "ui"))]
pub fn setup() -> (Option<std::marker::PhantomData<dyn std::any::Any>>, Option<std::marker::PhantomData<dyn std::any::Any>>, Arc<PaintLogsCallbacks>) {
    let logging = Arc::new(define_log_callbacks());
    (None, None, logging)
}
