// spawns the hyper server on a separate thread
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use super::fips;
use super::PaintLogsCallbacks;
use configuration::configuration::Configuration;
use plugin_registry::ExternalFunctions;
use utility::log::Loggable;
use std::marker::PhantomData;
use std::any::Any;
use log::info;

pub fn spawn_backend(
    configuration: &Arc<Mutex<Configuration>>,
    plugins: &Arc<Mutex<ExternalFunctions>>,
    addr: &SocketAddr,
    logger: &Arc<PaintLogsCallbacks>,
) -> JoinHandle<hyper::Result<()>> {
    let capture_plugins = plugins.clone();
    let capture_configuration = configuration.clone();
    let capture_logger = logger.clone();

    let make_svc = make_service_fn(move |_| {
        let inner_plugins = capture_plugins.clone();
        let inner_configuration = capture_configuration.clone();
        let inner_logger = capture_logger.clone();

        let responder = Box::new(move |req: Request<Body>| {
            let innermost_plugins = inner_plugins.clone();
            let innermost_configuration = inner_configuration.clone();
            let innermost_logger = inner_logger.clone();

            async move {
                fips::routes(
                    req,
                    innermost_configuration,
                    innermost_plugins,
                    &innermost_logger,
                )
                .await
            }
        });
        let service = service_fn(responder);

        async move { Ok::<_, hyper::Error>(service) }
    });

    tokio::spawn(Server::bind(addr).serve(make_svc))
}

#[cfg(not(feature = "ui"))]
fn define_log_callbacks() -> PaintLogsCallbacks {
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
