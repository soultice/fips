use super::request::pimps;
use crate::debug::{PimpsInfo, PrintInfo, RequestInfo, ResponseInfo, TrafficInfo};
use crate::State;
use hyper::header::HeaderValue;
use hyper::{Body, Method, Request, Response, StatusCode};
use std::sync::Arc;

pub async fn routes(req: Request<Body>, state: Arc<State>) -> Result<Response<Body>, hyper::Error> {
    let req_info = RequestInfo::from(&req);

    state
        .add_traffic_info(TrafficInfo::IncomingRequest(req_info))
        .unwrap_or_default();

    let matching_rules = state
        .configuration
        .lock()
        .unwrap()
        .active_matching_rules(req.uri().path(), req.method());

    match matching_rules.len() {
        0 => {
            let mut no_matching_rule = Response::new(Body::from("no matching rule found"));
            *no_matching_rule.status_mut() = StatusCode::NOT_FOUND;
            state
                .add_message(PrintInfo::PLAIN(format!(
                    "No matching rule found for URI: {}",
                    &req.uri()
                )))
                .unwrap_or_default();
            Ok(no_matching_rule)
        }

        _ => match (req.method(), req.uri().path()) {
            (&Method::GET, "/favicon.ico") => Ok(Response::new(Body::from(""))),

            (&Method::OPTIONS, _) => {
                let mut preflight = Response::new(Body::from(""));
                preflight
                    .headers_mut()
                    .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
                preflight.headers_mut().insert(
                    "Access-Control-Allow-Headers",
                    HeaderValue::from_static("*"),
                );
                preflight.headers_mut().insert(
                    "Access-Control-Allow-Methods",
                    HeaderValue::from_static("*"),
                );
                Ok(preflight)
            }

            _ => {
                let mut first_matched_rule = state
                    .configuration
                    .lock()
                    .unwrap()
                    .clone_rule(matching_rules[0]);

                let (parts, body) = req.into_parts();

                let resp: Response<Body> = pimps(body, parts, &state, &mut first_matched_rule)
                    .await
                    .unwrap();

                let response_info = ResponseInfo::from(&resp);

                state
                    .add_traffic_info(TrafficInfo::OutgoingResponse(response_info))
                    .unwrap_or_default();

                if let Some(sleep) = first_matched_rule.sleep {
                    tokio::time::sleep(tokio::time::Duration::from_millis(sleep)).await;
                }
                Ok(resp)
            }
        },
    }
}
