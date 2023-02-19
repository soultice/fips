use crate::PaintLogsCallbacks;

use super::request::handle_mode;

use configuration::Configuration;
use hyper::{
    body::Bytes,
    header::{HeaderMap, HeaderValue},
    http::request::Parts,
    Body, Method, Request, Response, StatusCode, Uri,
};
use plugin_registry::ExternalFunctions;
use std::sync::{Arc, Mutex, MutexGuard};

use terminal_ui::{debug::PrintInfo};

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

// this should be segmented with better care, split into smaller functions, move everything possible from state to separate arguments
pub async fn routes<'a>(
    req: Request<Body>,
    configuration: Arc<Mutex<Configuration>>,
    plugins: Arc<Mutex<ExternalFunctions>>,
    logging: &PaintLogsCallbacks<'a>,
) -> Result<Response<Body>, hyper::Error> {
    (logging.log_incoming_request_to_fips)(&req);

    let split = SplitRequest::new(req).await;

    if split.method == Method::OPTIONS {
        let mut preflight = Response::new(Body::default());
        add_cors_headers(preflight.headers_mut());
        return Ok(preflight);
    }

    let matching_rules = configuration.lock().unwrap().active_matching_rules(
        split.uri.path(),
        &split.method,
        &split.body_text,
    );

    match matching_rules.len() {
        0 => {
            let mut no_matching_rule = Response::new(Body::from("no matching rule found"));
            *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;
            add_cors_headers(no_matching_rule.headers_mut());
            (logging.log_plain)(format!("No matching rule found for URI: {}", &split.uri));
            Ok(no_matching_rule)
        }

        _ => match (&split.method, split.uri.path()) {
            (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::default())),

            _ => {
                let mut first_matched_rule = configuration
                    .lock()
                    .unwrap()
                    .clone_rule(matching_rules[0]);

                let resp: Response<Body> = handle_mode(
                    split.body_bytes,
                    split.parts,
                    &plugins,
                    &mut first_matched_rule,
                    logging,
                )
                .await
                .unwrap();

                (logging.log_outgoing_response_to_client)(&resp);

                if let Some(sleep) = first_matched_rule.sleep {
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
