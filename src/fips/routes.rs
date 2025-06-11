use crate::{
    configuration::{
        configuration::Config,
        intermediary::{Intermediary, IntermediaryError, into_response},
    },
    utility::log::{Loggable, LoggableType, RequestInfo, ResponseInfo},
    plugin_registry::ExternalFunctions,
};

use hyper::{
    header::{HeaderMap, HeaderValue},
    Request, Response, StatusCode,
    http::Method,
};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use http_body_util::combinators::BoxBody;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

use eyre::{Result};
use thiserror::Error;
use crate::configuration::rule::error::ConfigurationError;

type BoxedBody = BoxBody<Bytes, hyper::Error>;

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("HTTP error: {0}")]
    Hyper(#[from] hyper::Error),
    
    #[error("HTTP build error: {0}")]
    HttpBuild(#[from] hyper::http::Error),
    
    #[error("Client error: {0}")]
    Client(#[from] hyper_util::client::legacy::Error),
    
    #[error("Intermediary error: {0}")]
    Intermediary(#[from] IntermediaryError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Rule error: {0}")]
    RuleError(#[from] eyre::Error),
    
    #[error("Other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

// Helper function for empty responses
fn empty_response(status: StatusCode) -> Response<BoxedBody> {
    Response::builder()
        .status(status)
        .body(Empty::new().map_err(|never| match never {}).boxed())
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Empty::new().map_err(|never| match never {}).boxed())
                .unwrap()
        })
}

// Helper function to collect body bytes
async fn collect_body_bytes(body: hyper::body::Incoming) -> Result<String, RoutingError> {
    let bytes = http_body_util::BodyExt::collect(body)
        .await?
        .to_bytes();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// Helper function to create an initial intermediary from request parts
fn create_intermediary_from_parts(parts: hyper::http::request::Parts) -> Result<Intermediary, RoutingError> {
    Ok(Intermediary {
        uri: Some(parts.uri),
        method: Some(parts.method),
        status: StatusCode::OK,
        headers: parts.headers,
        body: None,
    })
}

// Helper function to create a boxed body with hyper::Error
fn create_boxed_body(data: Option<String>) -> BoxedBody {
    match data {
        Some(body) => Full::new(Bytes::from(body)).map_err(|never| match never {}).boxed(),
        None => Empty::new().map_err(|never| match never {}).boxed(),
    }
}

impl From<ConfigurationError> for RoutingError {
    fn from(err: ConfigurationError) -> Self {
        RoutingError::RuleError(err.into())
    }
}

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    configuration: Arc<AsyncMutex<Config>>,
    paint_logs: Arc<AsyncMutex<Vec<Box<dyn Fn(Loggable) + Send + Sync>>>>,
    external_functions: &ExternalFunctions,
) -> Result<Response<BoxedBody>, RoutingError> {
    let requestinfo = RequestInfo::from(&req);
    let request_id = requestinfo.id.clone();

    let log_output = Loggable {
        message_type: LoggableType::IncomingRequestAtFips(requestinfo),
        message: format!("Incoming Request at FIPS with id: {}", request_id),
    };

    // Log the incoming request
    for callback in paint_logs.lock().await.iter() {
        callback(log_output.clone());
    }

    let (parts, body) = req.into_parts();
    let body_str = collect_body_bytes(body).await?;
    
    let mut intermediary = create_intermediary_from_parts(parts)?;
    intermediary.body = Some(body_str);

    // Get the configuration and check rules
    let config = configuration.lock().await;
    match config.check_rule(&intermediary).await {
        Ok(mut container) => {
            // Apply the rules in the container to modify the intermediary
            container.apply(&mut intermediary, external_functions).await?;
            
            // Convert the intermediary to a request
            let mut request_builder = Request::builder()
                .method(intermediary.method.unwrap_or(Method::GET))
                .uri(intermediary.uri.unwrap_or_else(|| "/".parse().unwrap()));

            if let Some(headers) = request_builder.headers_mut() {
                headers.extend(intermediary.headers);
            }

            let request = request_builder
                .body(create_boxed_body(intermediary.body))?;

            // Make the request using a properly typed client
            let client: Client<HttpConnector, BoxedBody> = Client::builder(TokioExecutor::new())
                .build(HttpConnector::new());
            
            let resp = client.request(request).await?;
            let mut intermediary = Intermediary {
                status: resp.status(),
                headers: resp.headers().clone(),
                body: None,
                uri: None,
                method: None,
            };

            // Convert the intermediary to a response
            let response = into_response(intermediary)?;

            // Create response info for logging
            let response_info = ResponseInfo::from(&response);
            
            let log_output = Loggable {
                message_type: LoggableType::OutgoingResponseAtFips(response_info),
                message: "Response sent".to_owned(),
            };

            // Log the outgoing response
            for callback in paint_logs.lock().await.iter() {
                callback(log_output.clone());
            }

            // Convert incoming response to intermediary and apply rules
            let mut inter = Intermediary::new();
            inter.status = resp.status();
            inter.headers = resp.headers().clone();

            // Convert body and apply rules
            let body_bytes = http_body_util::BodyExt::collect(resp.into_body())
                .await?
                .to_bytes();
            
            // First try to parse as string
            inter.body = Some(String::from_utf8_lossy(&body_bytes).into_owned());

            // Apply rules from container if needed
            container.apply(&mut inter, external_functions).await?;

            // Convert intermediary back to response and add CORS headers
            let mut resp = Response::builder()
                .status(inter.status)
                .body({
                    if let Some(body) = inter.body {
                        Full::new(Bytes::from(body))
                            .map_err(|never| match never {})
                            .boxed()
                    } else {
                        Empty::new()
                            .map_err(|never| match never {})
                            .boxed()
                    }
                })
                .map_err(|e| RoutingError::HttpBuild(e))?;
            
            // Copy headers from intermediary
            resp.headers_mut().extend(inter.headers);
            add_cors_headers(resp.headers_mut());

            Ok(resp)
        },
        Err(_) => {
            Ok(empty_response(StatusCode::NOT_FOUND))
        }
    }
}

fn add_cors_headers(headers: &mut HeaderMap<HeaderValue>) {
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert("Access-Control-Allow-Methods", HeaderValue::from_static("*"));
    headers.insert("Access-Control-Allow-Headers", HeaderValue::from_static("*"));
}
