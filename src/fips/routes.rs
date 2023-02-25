use crate::PaintLogsCallbacks;

use super::request::handle_mode;

use configuration::configuration::Configuration;
use hyper::{
    body::Bytes,
    header::{HeaderMap, HeaderValue},
    http::request::Parts,
    Body, Method, Request, Response, StatusCode, Uri,
};
use plugin_registry::ExternalFunctions;
use std::sync::{Arc, Mutex};

use utility::log::{Loggable, LoggableType, RequestInfo, ResponseInfo};

struct SplitRequest {
    uri: Uri,
    method: Method,
    body_text: String,
    body_bytes: Bytes,
    parts: Parts,
}

impl SplitRequest {
    async fn new(req: Request<Body>) -> Self {
        let (parts, body) = req.into_parts();
        {
            let uri = parts.uri.clone();
            let method = parts.method.clone();
            let body_bytes = hyper::body::to_bytes(body).await.unwrap();
            let body_text = String::from_utf8(body_bytes.to_vec()).unwrap();
            SplitRequest {
                uri,
                method,
                body_text,
                body_bytes,
                parts,
            }
        }
    }
}

// this should be segmented with better care, split into smaller functions, move everything possible from state to separate arguments
pub async fn routes(
    req: Request<Body>,
    configuration: Arc<Mutex<Configuration>>,
    plugins: Arc<Mutex<ExternalFunctions>>,
    logging: &Arc<PaintLogsCallbacks>,
) -> Result<Response<Body>, hyper::Error> {
    let requestinfo = RequestInfo::from(&req);
    let log_output = Loggable {
        message_type: LoggableType::IncomingRequestAtFfips(requestinfo),
        message: "".to_owned(),
    };
    (logging.0)(&log_output);

    let split = SplitRequest::new(req).await;

    if split.method == Method::OPTIONS {
        let mut preflight = Response::new(Body::default());
        add_cors_headers(preflight.headers_mut());
        return Ok(preflight);
    }

    let matching_rules = configuration.lock().unwrap().get_active_matching_rules(
        split.uri.path(),
        &split.method,
        &split.body_text,
    );

    match matching_rules.len() {
        0 => {
            let mut no_matching_rule = Response::new(Body::from("no matching rule found"));
            *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;
            add_cors_headers(no_matching_rule.headers_mut());
            (logging.0)(&Loggable {
                message: format!("No matching rule found for URI: {}", &split.uri),
                message_type: LoggableType::Plain,
            });
            Ok(no_matching_rule)
        }

        _ => match (&split.method, split.uri.path()) {
            (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::default())),

            _ => {
                let mut first_matched_rule =
                    configuration.lock().unwrap().clone_rule(matching_rules[0]);

                let resp: Response<Body> = handle_mode(
                    split.body_bytes,
                    split.parts,
                    &plugins,
                    &mut first_matched_rule,
                    logging,
                )
                .await
                .unwrap();

                let responseinfo = ResponseInfo::from(&resp);
                let log_output = Loggable {
                    message_type: LoggableType::OutGoingResponseFromFips(responseinfo),
                    message: "".to_owned(),
                };
                (logging.0)(&log_output);

                let sleep_time = first_matched_rule.get_sleep();
                if let Some(sleep) = sleep_time{
                    tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
                }
                Ok(resp)
            }
        },
    }
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    headers.insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );
    headers.insert(
        "Access-Control-Allow-Methods",
        HeaderValue::from_static("*"),
    );
}
