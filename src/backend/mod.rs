// spawns the hyper server on a separate thread
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::Request;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::fips;
use super::PaintLogsCallbacks;
use crate::configuration::configuration::Config;
use tokio::sync::Mutex as AsyncMutex;

#[cfg(not(feature = "ui"))]
use std::marker::PhantomData;
#[cfg(not(feature = "ui"))]
use std::any::Any;
#[cfg(not(feature = "ui"))]
use log::info;

pub fn spawn_backend(
    configuration: &Arc<AsyncMutex<Config>>,
    addr: &SocketAddr,
    logger: &Arc<PaintLogsCallbacks>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    let capture_configuration = configuration.clone();
    let capture_logger = logger.clone();
    let addr = *addr;

    tokio::spawn(async move {
        let listener = TcpListener::bind(addr).await?;
        
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            
            let config = capture_configuration.clone();
            let logger = capture_logger.clone();
            
            tokio::task::spawn(async move {
                let service = service_fn(move |req: Request<Incoming>| {
                    let config = config.clone();
                    let logger = logger.clone();
                    async move {
                        fips::routes(req, config, &logger).await
                    }
                });
                
                if let Err(err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                    .serve_connection(io, service)
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    })
}

#[cfg(not(feature = "ui"))]
fn define_log_callbacks() -> PaintLogsCallbacks {
    use crate::utility::log::Loggable;

    let log = Box::new(|message: &Loggable| info!("{:?}", message.message));
    PaintLogsCallbacks(log)
}

#[cfg(not(feature = "ui"))]
pub fn setup() -> (Option<PhantomData<dyn Any>>, Option<PhantomData<dyn Any>>, Arc<PaintLogsCallbacks>) {
    let logging = Arc::new(define_log_callbacks());
    (
        None,
        None,
        logging,
    )
}
